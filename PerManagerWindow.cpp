#include "PetManagerWindow.h"
#include "PetWindow.h"
#include "resource.h"
#include <commdlg.h>
#include <string>
#include "resource.h"
using std::wstring;


static HWND petHwnd = nullptr;
static std::wstring selectedImagePath = L"";

#define WINDOW_CLASS_NAME L"DesktopPetManagerClass"


void PetManagerWindow::Init(HINSTANCE hInstance) {
    //DialogBoxParam(hInstance, MAKEINTRESOURCE(IDD_PET_MANAGER), NULL, DialogProc, 0);
   /* INT_PTR result = DialogBoxParam(hInstance, MAKEINTRESOURCE(IDD_PET_MANAGER), NULL, DialogProc, 0);
    if (result == -1) {
        MessageBox(NULL, L"Failed to load Pet Manager dialog box!", L"Error", MB_OK | MB_ICONERROR);
    }*/

    hInst = hInstance;

    WNDCLASSEX wc = { sizeof(WNDCLASSEX) };
    wc.style = CS_HREDRAW | CS_VREDRAW;
    wc.lpfnWndProc = PetManagerWindow::WndProc;
    wc.hInstance = hInstance;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    wc.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
    wc.lpszClassName = WINDOW_CLASS_NAME;

    RegisterClassEx(&wc);

    hwndMain = CreateWindowEx(
        0,
        WINDOW_CLASS_NAME,
        L"Desktop Pet Manager",
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT, CW_USEDEFAULT, 400, 200,
        NULL, NULL, hInstance, this
    );

}

void PetManagerWindow::Show() {
    ShowWindow(hwndMain, SW_SHOW);
    UpdateWindow(hwndMain);
}

LRESULT CALLBACK PetManagerWindow::WndProc(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    PetManagerWindow* pThis;

    if (msg == WM_NCCREATE) {
        pThis = static_cast<PetManagerWindow*>(((LPCREATESTRUCT)lParam)->lpCreateParams);
        SetWindowLongPtr(hWnd, GWLP_USERDATA, (LONG_PTR)pThis);
    }
    else {
        pThis = (PetManagerWindow*)GetWindowLongPtr(hWnd, GWLP_USERDATA);
    }

    if (!pThis) return DefWindowProc(hWnd, msg, wParam, lParam);

    switch (msg) {
    case WM_CREATE:
        pThis->CreateControls(hWnd);
        break;

    case WM_COMMAND:
        switch (LOWORD(wParam)) {
        case 1: pThis->OnBrowse(hWnd); break;
        case 2: pThis->OnStart(hWnd); break;
        case 3: pThis->OnStop(hWnd); break;
        }
        break;

    case WM_DESTROY:
        if (petHwnd) DestroyWindow(petHwnd);
        PostQuitMessage(0);
        break;
    }

    return DefWindowProc(hWnd, msg, wParam, lParam);
}

void PetManagerWindow::CreateControls(HWND hWnd) {
    hwndEdit = CreateWindowW(L"EDIT", L"",
        WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL,
        10, 20, 260, 25,
        hWnd, (HMENU)10, hInst, NULL);

    hwndBtnBrowse = CreateWindowW(L"BUTTON", L"Browse...",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        280, 20, 90, 25,
        hWnd, (HMENU)1, hInst, NULL);

    hwndBtnStart = CreateWindowW(L"BUTTON", L"Start Pet",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        60, 60, 100, 30,
        hWnd, (HMENU)2, hInst, NULL);

    hwndBtnStop = CreateWindowW(L"BUTTON", L"Stop Pet",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        180, 60, 100, 30,
        hWnd, (HMENU)3, hInst, NULL);
}

void PetManagerWindow::OnBrowse(HWND hWnd) {
    WCHAR filePath[MAX_PATH] = {};
    OPENFILENAME ofn = { sizeof(ofn) };
    ofn.hwndOwner = hWnd;
    ofn.lpstrFilter = L"Images\0*.png;*.bmp;*.jpg;*.gif\0All Files\0*.*\0";
    ofn.lpstrFile = filePath;
    ofn.nMaxFile = MAX_PATH;
    ofn.Flags = OFN_FILEMUSTEXIST;

    if (GetOpenFileName(&ofn)) {
        SetWindowText(hwndEdit, filePath);
        selectedImagePath = filePath;
    }
}

void PetManagerWindow::OnStart(HWND hWnd) {
   if (selectedImagePath.empty()) {
        MessageBox(hWnd, L"Please select a pet image first.", L"Error", MB_OK);
        return;
    }
    if (!petHwnd) {
        petHwnd = CreatePetWindow(selectedImagePath.c_str(), 128, 128);
    }
}

void PetManagerWindow::OnStop(HWND) {
    if (petHwnd) {
        DestroyWindow(petHwnd);
        petHwnd = nullptr;
    }
}