// DesktopPet.cpp : Defines the entry point for the application.

#include <windows.h>       // include this FIRST
#include <objidl.h>        // required by GDI+
#include <gdiplus.h>       // now safe to include
#pragma comment(lib, "Gdiplus.lib")
using namespace Gdiplus;

#include "framework.h"
#include "DesktopPet.h"

#define MAX_LOADSTRING 100

// Global Variables:
HINSTANCE hInst;                                // current instance
WCHAR szTitle[MAX_LOADSTRING];                  // The title bar text
WCHAR szWindowClass[MAX_LOADSTRING];

ULONG_PTR gdiplusToken;
Image* petImage = nullptr;
int petX = 100, petY = 100; // Position of the pet

bool isDragging = false;
POINT dragOffset = { 0, 0 };


// the main window class name

// Forward declarations of functions included in this code module:
ATOM                MyRegisterClass(HINSTANCE hInstance);
BOOL                InitInstance(HINSTANCE, int);
LRESULT CALLBACK    WndProc(HWND, UINT, WPARAM, LPARAM);
INT_PTR CALLBACK    About(HWND, UINT, WPARAM, LPARAM);

int APIENTRY wWinMain(_In_ HINSTANCE hInstance,
                     _In_opt_ HINSTANCE hPrevInstance,
                     _In_ LPWSTR    lpCmdLine,
                     _In_ int       nCmdShow)
{
    UNREFERENCED_PARAMETER(hPrevInstance);
    UNREFERENCED_PARAMETER(lpCmdLine);

    // TODO: Place code here
    GdiplusStartupInput gsi;
    GdiplusStartup(&gdiplusToken, &gsi, NULL);


    // Initialize global strings
    LoadStringW(hInstance, IDS_APP_TITLE, szTitle, MAX_LOADSTRING);
    LoadStringW(hInstance, IDC_DESKTOPPET, szWindowClass, MAX_LOADSTRING);
    MyRegisterClass(hInstance);

    // Perform application initialization:
    if (!InitInstance (hInstance, nCmdShow))
    {
        return FALSE;
    }

    HACCEL hAccelTable = LoadAccelerators(hInstance, MAKEINTRESOURCE(IDC_DESKTOPPET));

    MSG msg;

    // Main message loop:
    while (GetMessage(&msg, nullptr, 0, 0))
    {
        if (!TranslateAccelerator(msg.hwnd, hAccelTable, &msg))
        {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        }
    }

    delete petImage;
    GdiplusShutdown(gdiplusToken);


    return (int) msg.wParam;
}


void DrawPet(HWND hWnd)
{
    if (!petImage)
        return;

    UINT width = petImage->GetWidth();
    UINT height = petImage->GetHeight();

    HDC hdcScreen = GetDC(NULL);
    HDC hdcMem = CreateCompatibleDC(hdcScreen);

    BITMAPINFO bmi = { 0 };
    bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bmi.bmiHeader.biWidth = width;
    bmi.bmiHeader.biHeight = -((LONG)height); // top-down DIB
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    void* bits = nullptr;
    HBITMAP hBitmap = CreateDIBSection(hdcScreen, &bmi, DIB_RGB_COLORS, &bits, NULL, 0);
    SelectObject(hdcMem, hBitmap);

    Graphics graphics(hdcMem);
    graphics.SetCompositingMode(CompositingModeSourceOver);
    graphics.SetSmoothingMode(SmoothingModeHighQuality);
    graphics.SetInterpolationMode(InterpolationModeHighQualityBicubic);

    graphics.Clear(Color(0, 0, 0, 0));
    //graphics.DrawImage(petImage, 0, 0, width, height);
    graphics.DrawImage(petImage, 0, 0, 128, 128);


    SIZE size = { (LONG)width, (LONG)height };
    POINT ptSrc = { 0, 0 };
    POINT ptWinPos = { petX, petY };

    BLENDFUNCTION blend = {};
    blend.BlendOp = AC_SRC_OVER;
    blend.SourceConstantAlpha = 255;
    blend.AlphaFormat = AC_SRC_ALPHA;

    UpdateLayeredWindow(hWnd, hdcScreen, &ptWinPos, &size, hdcMem, &ptSrc, 0, &blend, ULW_ALPHA);

    DeleteObject(hBitmap);
    DeleteDC(hdcMem);
    ReleaseDC(NULL, hdcScreen);
}




//
//  FUNCTION: MyRegisterClass()
//
//  PURPOSE: Registers the window class.
//
ATOM MyRegisterClass(HINSTANCE hInstance)
{
    WNDCLASSEXW wcex;

    wcex.cbSize = sizeof(WNDCLASSEX);

    wcex.style          = CS_HREDRAW | CS_VREDRAW;
    wcex.lpfnWndProc    = WndProc;
    wcex.cbClsExtra     = 0;
    wcex.cbWndExtra     = 0;
    wcex.hInstance      = hInstance;
    wcex.hIcon          = LoadIcon(hInstance, MAKEINTRESOURCE(IDI_DESKTOPPET));
    wcex.hCursor        = LoadCursor(nullptr, IDC_ARROW);
    wcex.hbrBackground = nullptr; //(HBRUSH)(COLOR_WINDOW+1);
    wcex.lpszMenuName   = MAKEINTRESOURCEW(IDC_DESKTOPPET);
    wcex.lpszClassName  = szWindowClass;
    wcex.hIconSm        = LoadIcon(wcex.hInstance, MAKEINTRESOURCE(IDI_SMALL));

    return RegisterClassExW(&wcex);
}

//
//   FUNCTION: InitInstance(HINSTANCE, int)
//
//   PURPOSE: Saves instance handle and creates main window
//
//   COMMENTS:
//
//        In this function, we save the instance handle in a global variable and
//        create and display the main program window.
//
BOOL InitInstance(HINSTANCE hInstance, int nCmdShow)
{
   hInst = hInstance; // Store instance handle in our global variable

   /*HWND hWnd = CreateWindowW(szWindowClass, szTitle, WS_OVERLAPPEDWINDOW,
      CW_USEDEFAULT, 0, CW_USEDEFAULT, 0, nullptr, nullptr, hInstance, nullptr);*/

   HWND hWnd = CreateWindowExW(
       WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW, //| WS_EX_TRANSPARENT,
       szWindowClass, szTitle, WS_POPUP,
       petX, petY, 128, 128,
       nullptr, nullptr, hInstance, nullptr);

   //SetLayeredWindowAttributes(hWnd, RGB(255, 0, 255), 0, LWA_COLORKEY);
   //SetLayeredWindowAttributes(hWnd, 0, 255, LWA_ALPHA);  // Fully opaque, not colorkey


   if (!hWnd)
   {
      return FALSE;
   }

   ShowWindow(hWnd, nCmdShow);
   UpdateWindow(hWnd);

   petImage = new Image(L"D:\\Personal\\DesktopPet\\x64\\Debug\\lion.png");

   if (!petImage || petImage->GetLastStatus() != Ok) {
       MessageBox(NULL, L"Failed to load image!", L"Error", MB_OK);
   }


   if (petImage->GetLastStatus() != Ok) {
       MessageBox(NULL, L"Failed to load image", L"Error", MB_OK);
   }
   else {
       DrawPet(hWnd);  // Only draw after image is ready
   }



   return TRUE;
}

//
//  FUNCTION: WndProc(HWND, UINT, WPARAM, LPARAM)
//
//  PURPOSE: Processes messages for the main window.
//
//  WM_COMMAND  - process the application menu
//  WM_PAINT    - Paint the main window
//  WM_DESTROY  - post a quit message and return
//
//
LRESULT CALLBACK WndProc(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam)
{
    switch (message)
    {
    case WM_LBUTTONDOWN:
    {
        SetCapture(hWnd);
        isDragging = true;

        POINT pt;
        GetCursorPos(&pt);

        RECT rect;
        GetWindowRect(hWnd, &rect);

        dragOffset.x = pt.x - rect.left;
        dragOffset.y = pt.y - rect.top;
        return 0;
    }

    case WM_MOUSEMOVE:
    {
        if (isDragging)
        {
            POINT pt;
            GetCursorPos(&pt);

            petX = pt.x - dragOffset.x;
            petY = pt.y - dragOffset.y;

            DrawPet(hWnd);  // Redraw at new position
        }
        return 0;
    }

    case WM_LBUTTONUP:
    {
        if (isDragging)
        {
            isDragging = false;
            ReleaseCapture();
        }
        return 0;
    }


    case WM_COMMAND:
        {
            int wmId = LOWORD(wParam);
            // Parse the menu selections:
            switch (wmId)
            {
            case IDM_ABOUT:
                DialogBox(hInst, MAKEINTRESOURCE(IDD_ABOUTBOX), hWnd, About);
                break;
            case IDM_EXIT:
                DestroyWindow(hWnd);
                break;
            default:
                return DefWindowProc(hWnd, message, wParam, lParam);
            }
        }
        break;
    case WM_PAINT:
        {
            
        PAINTSTRUCT ps;
        BeginPaint(hWnd, &ps);
        EndPaint(hWnd, &ps);
        break;
        }
    case WM_DESTROY:
        PostQuitMessage(0);
        break;
    default:
        return DefWindowProc(hWnd, message, wParam, lParam);
    }
    return 0;
}

// Message handler for about box.
INT_PTR CALLBACK About(HWND hDlg, UINT message, WPARAM wParam, LPARAM lParam)
{
    UNREFERENCED_PARAMETER(lParam);
    switch (message)
    {
    case WM_INITDIALOG:
        return (INT_PTR)TRUE;

    case WM_COMMAND:
        if (LOWORD(wParam) == IDOK || LOWORD(wParam) == IDCANCEL)
        {
            EndDialog(hDlg, LOWORD(wParam));
            return (INT_PTR)TRUE;
        }
        break;
    }
    return (INT_PTR)FALSE;
}
