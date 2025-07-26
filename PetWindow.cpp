#include "PetWindow.h"
//#include <string>
//#include <vector>


static Image* petImage = nullptr;
static int petX = 100, petY = 100;
static HWND hwndGlobal = nullptr;

//static std::vector<Gdiplus::Image*> walkFrames;
//static int currentFrame = 0;
//static int petWidth = 128;
//static int petHeight = 128;
//static UINT_PTR animationTimerId = 1;
//static const int FRAME_INTERVAL_MS = 100; // milliseconds

LRESULT CALLBACK PetWndProc(HWND, UINT, WPARAM, LPARAM);

HWND CreatePetWindow(const wchar_t* imagePath, int width, int height) {
    petImage = new Image(imagePath);

    // Clear previous frames if any
    //for (auto* img : walkFrames) delete img;
    //walkFrames.clear();

    //// Load walk1.png to walk6.png
    //for (int i = 1; i <= 6; ++i) {
    //    std::wstring framePath = L"D:\\Personal\\DesktopPet\\assets\\walk" + std::to_wstring(i) + L".png";
    //    Gdiplus::Image* frame = new Gdiplus::Image(framePath.c_str());
    //    if (frame->GetLastStatus() == Ok) {
    //        walkFrames.push_back(frame);
    //    }
    //    else {
    //        MessageBox(NULL, (L"Failed to load frame: " + framePath).c_str(), L"Error", MB_OK);
    //        delete frame;
    //    }
    //}


    if (!petImage || petImage->GetLastStatus() != Ok) return nullptr;

    const wchar_t* CLASS_NAME = L"PetWindowClass";
    WNDCLASS wc = { };
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
    if (!petImage) return;

    UINT width = petImage->GetWidth();
    UINT height = petImage->GetHeight();

    //if (walkFrames.empty()) return;

    /*UINT width = petWidth;
    UINT height = petHeight;*/

    HDC hdcScreen = GetDC(NULL);
    HDC hdcMem = CreateCompatibleDC(hdcScreen);

    BITMAPINFO bmi = { 0 };
    bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bmi.bmiHeader.biWidth = width;
    bmi.bmiHeader.biHeight = -((LONG)height);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    void* bits = nullptr;
    HBITMAP hBitmap = CreateDIBSection(hdcScreen, &bmi, DIB_RGB_COLORS, &bits, NULL, 0);
    SelectObject(hdcMem, hBitmap);

    Graphics graphics(hdcMem);
    graphics.SetCompositingMode(CompositingModeSourceOver);
    graphics.Clear(Color(0, 0, 0, 0));
    graphics.DrawImage(petImage, 0, 0, 128, 128);
    //graphics.DrawImage(walkFrames[currentFrame], 0, 0, petWidth, petHeight);


    POINT ptSrc = { 0, 0 };
    SIZE sizeWindow = { (LONG)width, (LONG)height };
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
        // SetTimer(hwnd, animationTimerId, FRAME_INTERVAL_MS, NULL);
        DrawPet(hwnd);
        return 0;
    /*case WM_TIMER:
        if (!walkFrames.empty()) {
            currentFrame = (currentFrame + 1) % walkFrames.size();
            DrawPet(hwnd);
        }
        return 0;*/

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
        delete petImage;
        petImage = nullptr;

        /*KillTimer(hwnd, animationTimerId);

        for (auto* img : walkFrames) {
            delete img;
        }
        walkFrames.clear();*/

        return 0;
    }
    return DefWindowProc(hwnd, msg, wParam, lParam);
}
