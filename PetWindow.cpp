#include "PetWindow.h"
#include <string>
#include <vector>
#include <gdiplus.h>

using namespace Gdiplus;
// Animation timing in milliseconds
static const int FRAME_INTERVAL_MS = 100; // 100ms per frame (10 FPS)


static std::vector<Image*> walkFrames;
static int currentFrame = 0;
static int petX = 100, petY = 100;
static int petWidth = 128, petHeight = 128;
static bool isStatic = false;
static UINT_PTR animationTimerId = 1;
static HWND hwndGlobal = nullptr;

LRESULT CALLBACK PetWndProc(HWND, UINT, WPARAM, LPARAM);

// Utility: create and show the window
static HWND CreatePetWindowInternal(int width, int height) {
    const wchar_t* CLASS_NAME = L"PetWindowClass";
    WNDCLASS wc = {};
    wc.lpfnWndProc = PetWndProc;
    wc.hInstance = GetModuleHandle(NULL);
    wc.lpszClassName = CLASS_NAME;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    wc.hbrBackground = NULL;

    RegisterClass(&wc);

    HWND hwnd = CreateWindowEx(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        CLASS_NAME, L"Pet",
        WS_POPUP,
        petX, petY, width, height,
        NULL, NULL, wc.hInstance, NULL
    );

    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    UpdateWindow(hwnd);
    hwndGlobal = hwnd;
    return hwnd;
}

// Static image version
HWND CreateStaticPetWindow(const wchar_t* imagePath, int width, int height) {
    if (hwndGlobal) {
        DestroyWindow(hwndGlobal);   // Cleanly destroy the previous pet window
        hwndGlobal = nullptr;
    }

    for (auto* img : walkFrames) delete img;
    walkFrames.clear();

    Image* img = new Image(imagePath);
    if (img->GetLastStatus() != Ok) {
        MessageBox(NULL, L"Failed to load static image.", L"Error", MB_OK);
        delete img;
        return nullptr;
    }

    walkFrames.push_back(img);
    currentFrame = 0;
    isStatic = true;

    return CreatePetWindowInternal(width, height);
}

// Animation version
HWND CreatePetWindow(int width, int height, const std::wstring& path, bool isAnimation) {
    if (hwndGlobal) {
        DestroyWindow(hwndGlobal);   // Cleanly destroy the previous pet window
        hwndGlobal = nullptr;
    }
    
    for (auto* img : walkFrames) delete img;
    walkFrames.clear();

	currentFrame = 0;

    if (isAnimation) {
        for (int i = 1; i <= 6; ++i) {
            std::wstring framePath = path + L"\\walk" + std::to_wstring(i) + L".png";
            Gdiplus::Image* frame = new Gdiplus::Image(framePath.c_str());
            if (frame->GetLastStatus() == Gdiplus::Ok) {
                walkFrames.push_back(frame);
            }
            else {
                MessageBox(NULL, (L"Failed to load: " + framePath).c_str(), L"Load Error", MB_OK);
                delete frame;
            }
        }

        if (walkFrames.empty()) {
            MessageBox(NULL, L"No valid animation frames loaded.", L"Error", MB_OK);
            return nullptr;
        }

        isStatic = false;
    }
    else {
        Gdiplus::Image* img = new Gdiplus::Image(path.c_str());
        if (img->GetLastStatus() != Gdiplus::Ok) {
            MessageBox(NULL, L"Failed to load static image.", L"Error", MB_OK);
            delete img;
            return nullptr;
        }
        walkFrames.push_back(img);
        isStatic = true;
    }

    const wchar_t* CLASS_NAME = L"PetWindowClass";
    WNDCLASS wc = {};
    wc.lpfnWndProc = PetWndProc;
    wc.hInstance = GetModuleHandle(NULL);
    wc.lpszClassName = CLASS_NAME;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);

    RegisterClass(&wc);

    HWND hwnd = CreateWindowEx(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        CLASS_NAME, L"Pet",
        WS_POPUP,
        petX, petY, width, height,
        NULL, NULL, GetModuleHandle(NULL), NULL);

    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    UpdateWindow(hwnd);
    hwndGlobal = hwnd;
    return hwnd;
}


void DrawPet(HWND hwnd) {
    if (walkFrames.empty()) return;

    HDC hdcScreen = GetDC(NULL);
    HDC hdcMem = CreateCompatibleDC(hdcScreen);

    BITMAPINFO bmi = { 0 };
    bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bmi.bmiHeader.biWidth = petWidth;
    bmi.bmiHeader.biHeight = -petHeight;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    void* bits = nullptr;
    HBITMAP hBitmap = CreateDIBSection(hdcScreen, &bmi, DIB_RGB_COLORS, &bits, NULL, 0);
    SelectObject(hdcMem, hBitmap);

    Graphics graphics(hdcMem);
    graphics.SetCompositingMode(CompositingModeSourceOver);
    graphics.Clear(Color(0, 0, 0, 0));
    graphics.DrawImage(walkFrames[currentFrame], 0, 0, petWidth, petHeight);

    POINT ptSrc = { 0, 0 };
    SIZE sizeWindow = { petWidth, petHeight };
    POINT ptDst = { petX, petY };

    BLENDFUNCTION blend = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
    UpdateLayeredWindow(hwnd, hdcScreen, &ptDst, &sizeWindow, hdcMem, &ptSrc, 0, &blend, ULW_ALPHA);

    DeleteObject(hBitmap);
    DeleteDC(hdcMem);
    ReleaseDC(NULL, hdcScreen);
}

LRESULT CALLBACK PetWndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    static bool dragging = false;
    static POINT offset;

    switch (msg) {
    case WM_CREATE:
        if (!isStatic) {
            SetTimer(hwnd, animationTimerId, FRAME_INTERVAL_MS, NULL);
        }
        DrawPet(hwnd);
        return 0;

    case WM_TIMER:
        if (!walkFrames.empty() && !isStatic) {
            currentFrame = (currentFrame + 1) % walkFrames.size();
            DrawPet(hwnd);
        }
        return 0;

    case WM_LBUTTONDOWN:
        SetCapture(hwnd);
        dragging = true;
        {
            POINT pt;
            GetCursorPos(&pt);
            RECT rc;
            GetWindowRect(hwnd, &rc);
            offset.x = pt.x - rc.left;
            offset.y = pt.y - rc.top;
        }
        return 0;

    case WM_MOUSEMOVE:
        if (dragging) {
            POINT pt;
            GetCursorPos(&pt);
            petX = pt.x - offset.x;
            petY = pt.y - offset.y;
            DrawPet(hwnd);
        }
        return 0;

    case WM_LBUTTONUP:
        ReleaseCapture();
        dragging = false;
        return 0;

    case WM_DESTROY:
        KillTimer(hwnd, animationTimerId);
        for (auto* img : walkFrames) delete img;
        walkFrames.clear();
        return 0;
    }

    return DefWindowProc(hwnd, msg, wParam, lParam);
}
