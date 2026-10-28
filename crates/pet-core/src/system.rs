//! Watches CPU, memory, battery and disk readings and decides when the pet
//! should react. Readings come from the platform layer; this only smooths
//! them and applies thresholds with hysteresis and cooldowns.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub percent: f32,
    pub charging: bool,
}

/// One reading. Anything the platform cannot measure is `None`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub cpu_percent: Option<f32>,
    pub ram_percent: Option<f32>,
    pub battery: Option<Battery>,
    /// Free space on the system drive.
    pub disk_free_percent: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    pub cpu: f32,
    pub ram: f32,
    pub battery_low: f32,
    pub disk_low: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alert {
    CpuHigh(f32),
    RamHigh(f32),
    BatteryLow(f32),
    DiskLow(f32),
}

impl Alert {
    pub fn message(&self) -> String {
        match self {
            Alert::CpuHigh(p) => format!("Phew, it's hot in here! CPU at {p:.0}%."),
            Alert::RamHigh(p) => format!("Memory is getting full ({p:.0}% used)."),
            Alert::BatteryLow(p) => format!("Battery at {p:.0}%. Time to plug in?"),
            Alert::DiskLow(p) => format!("Only {p:.0}% disk space left."),
        }
    }
}

/// What the pet should look like right now.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub hot: bool,
    pub ram_full: bool,
    pub low_battery: bool,
    pub low_disk: bool,
}

/// Seconds between repeated alerts of one kind.
const COOLDOWN: [f64; 4] = [15.0 * 60.0, 15.0 * 60.0, 20.0 * 60.0, 2.0 * 3600.0];
/// How fast the CPU average follows readings (seconds).
const CPU_SMOOTHING: f32 = 20.0;

#[derive(Debug, Default)]
pub struct Watch {
    cpu_avg: Option<f32>,
    pub latest: Sample,
    pub condition: Condition,
    last_alert: [Option<f64>; 4],
}

impl Watch {
    pub fn cpu_average(&self) -> Option<f32> {
        self.cpu_avg
    }

    /// Feed one sample taken `dt` seconds after the previous one. Returns the
    /// alerts to announce (crossings, rate-limited).
    pub fn update(&mut self, s: Sample, dt: f32, clock: f64, lim: Limits) -> Vec<Alert> {
        self.latest = s;
        if let Some(cpu) = s.cpu_percent {
            let k = (dt / CPU_SMOOTHING).clamp(0.0, 1.0);
            self.cpu_avg = Some(match self.cpu_avg {
                Some(avg) => avg + (cpu - avg) * k,
                None => cpu,
            });
        }
        let mut alerts = Vec::new();
        let c = &mut self.condition;

        // Each limit turns on at the threshold and off a bit past it, so a
        // value hovering at the line does not flicker.
        if let Some(avg) = self.cpu_avg {
            let was = c.hot;
            c.hot = if was { avg > lim.cpu - 10.0 } else { avg >= lim.cpu };
            if c.hot && !was {
                alerts.push((0, Alert::CpuHigh(avg)));
            }
        }
        if let Some(ram) = s.ram_percent {
            let was = c.ram_full;
            c.ram_full = if was { ram > lim.ram - 5.0 } else { ram >= lim.ram };
            if c.ram_full && !was {
                alerts.push((1, Alert::RamHigh(ram)));
            }
        }
        match s.battery {
            Some(b) => {
                let was = c.low_battery;
                c.low_battery =
                    !b.charging && if was { b.percent < lim.battery_low + 5.0 } else { b.percent <= lim.battery_low };
                if c.low_battery && !was {
                    alerts.push((2, Alert::BatteryLow(b.percent)));
                }
            }
            None => c.low_battery = false,
        }
        if let Some(free) = s.disk_free_percent {
            let was = c.low_disk;
            c.low_disk = if was { free < lim.disk_low + 2.0 } else { free <= lim.disk_low };
            if c.low_disk && !was {
                alerts.push((3, Alert::DiskLow(free)));
            }
        }

        alerts
            .into_iter()
            .filter(|(i, _)| {
                let ok = self.last_alert[*i].is_none_or(|t| clock - t >= COOLDOWN[*i]);
                if ok {
                    self.last_alert[*i] = Some(clock);
                }
                ok
            })
            .map(|(_, a)| a)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIM: Limits = Limits { cpu: 90.0, ram: 90.0, battery_low: 20.0, disk_low: 10.0 };

    fn cpu(p: f32) -> Sample {
        Sample { cpu_percent: Some(p), ..Default::default() }
    }

    #[test]
    fn cpu_must_stay_high_to_count() {
        let mut w = Watch::default();
        w.update(cpu(10.0), 1.0, 0.0, LIM);
        // A short spike is smoothed away.
        for t in 1..4 {
            assert!(w.update(cpu(100.0), 1.0, t as f64, LIM).is_empty());
        }
        assert!(!w.condition.hot);
        // Sustained load triggers once.
        let mut alerts = vec![];
        for t in 4..120 {
            alerts.extend(w.update(cpu(100.0), 1.0, t as f64, LIM));
        }
        assert!(w.condition.hot);
        assert_eq!(alerts.len(), 1);
        assert!(matches!(alerts[0], Alert::CpuHigh(_)));
        // Cools down eventually.
        for t in 120..400 {
            w.update(cpu(5.0), 1.0, t as f64, LIM);
        }
        assert!(!w.condition.hot);
    }

    #[test]
    fn alerts_have_cooldown() {
        let mut w = Watch::default();
        let low = Sample { battery: Some(Battery { percent: 15.0, charging: false }), ..Default::default() };
        let ok = Sample { battery: Some(Battery { percent: 15.0, charging: true }), ..Default::default() };
        assert_eq!(w.update(low, 1.0, 0.0, LIM).len(), 1);
        assert!(w.update(ok, 1.0, 10.0, LIM).is_empty());
        assert!(!w.condition.low_battery);
        // Unplugged again soon after: no repeat nag.
        assert!(w.update(low, 1.0, 20.0, LIM).is_empty());
        assert!(w.condition.low_battery);
        w.update(ok, 1.0, 30.0, LIM);
        assert_eq!(w.update(low, 1.0, 30.0 + 21.0 * 60.0, LIM).len(), 1);
    }

    #[test]
    fn disk_and_ram_thresholds() {
        let mut w = Watch::default();
        let s = Sample { ram_percent: Some(95.0), disk_free_percent: Some(5.0), ..Default::default() };
        let a = w.update(s, 1.0, 0.0, LIM);
        assert_eq!(a.len(), 2);
        assert!(w.condition.ram_full && w.condition.low_disk);
        assert!(Alert::DiskLow(5.0).message().contains("5%"));
    }
}
