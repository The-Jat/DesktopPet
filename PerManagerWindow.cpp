#include "PetManagerWindow.h"
#include "PetWindow.h"
#include "resource.h"
#include <commdlg.h>
#include <string>
#include "resource.h"
#include <shlobj.h>        // For BROWSEINFO and SHBrowseForFolder
#include <shlwapi.h>       // Optional: for PathCombine, etc.
#pragma comment(lib, "shell32.lib") // Required for SHBrowseForFolder and SHGetPathFromIDList


using std::wstring;


static HWND petHwnd = nullptr;

static std::wstring selectedImagePath = L"";
static std::wstring selectedFolderPath = L"";

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

    /*case WM_COMMAND:
        switch (LOWORD(wParam)) {
        case 1: pThis->OnBrowse(hWnd); break;
        case 2: pThis->OnStart(hWnd); break;
        case 3: pThis->OnStop(hWnd); break;
        }
        break;*/
    case WM_COMMAND:
        if (LOWORD(wParam) == 4) {
            BOOL checked = SendMessage(pThis->hwndCheckAnimation, BM_GETCHECK, 0, 0);
            EnableWindow(pThis->hwndEditImage, !checked);
            EnableWindow(pThis->hwndBtnBrowseImage, !checked);
            EnableWindow(pThis->hwndEditFolder, checked);
            EnableWindow(pThis->hwndBtnBrowseFolder, checked);
        }

        switch (LOWORD(wParam)) {
        case 1: pThis->OnBrowseImage(hWnd); break;
        case 5: pThis->OnBrowseFolder(hWnd); break;
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
    // Checkbox: Use Animation
    hwndCheckAnimation = CreateWindowW(L"BUTTON", L"Use Animation",
        WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX,
        10, 10, 150, 20,
        hWnd, (HMENU)4, hInst, NULL);

    // Static Image Controls
    hwndEditImage = CreateWindowW(L"EDIT", L"",
        WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL,
        10, 40, 260, 25,
        hWnd, (HMENU)10, hInst, NULL);

    hwndBtnBrowseImage = CreateWindowW(L"BUTTON", L"Browse Image...",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        280, 40, 90, 25,
        hWnd, (HMENU)1, hInst, NULL);

    // Animation Folder Controls
    hwndEditFolder = CreateWindowW(L"EDIT", L"",
        WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL,
        10, 75, 260, 25,
        hWnd, (HMENU)11, hInst, NULL);

    hwndBtnBrowseFolder = CreateWindowW(L"BUTTON", L"Browse Folder...",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        280, 75, 90, 25,
        hWnd, (HMENU)5, hInst, NULL);

    // Start/Stop buttons
    hwndBtnStart = CreateWindowW(L"BUTTON", L"Start Pet",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        60, 115, 100, 30,
        hWnd, (HMENU)2, hInst, NULL);

    hwndBtnStop = CreateWindowW(L"BUTTON", L"Stop Pet",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        180, 115, 100, 30,
        hWnd, (HMENU)3, hInst, NULL);

    // Initially disable animation folder UI
    EnableWindow(hwndEditFolder, FALSE);
    EnableWindow(hwndBtnBrowseFolder, FALSE);
}


void PetManagerWindow::OnBrowseImage(HWND hWnd) {
    WCHAR filePath[MAX_PATH] = {};
    OPENFILENAME ofn = { sizeof(ofn) };
    ofn.hwndOwner = hWnd;
    ofn.lpstrFilter = L"Images\0*.png;*.bmp;*.jpg;*.gif\0All Files\0*.*\0";
    ofn.lpstrFile = filePath;
    ofn.nMaxFile = MAX_PATH;
    ofn.Flags = OFN_FILEMUSTEXIST;

    if (GetOpenFileName(&ofn)) {
        SetWindowText(hwndEditImage, filePath);
        selectedImagePath = filePath;
    }
}

void PetManagerWindow::OnBrowseFolder(HWND hWnd) {
    BROWSEINFO bi = { 0 };
    bi.lpszTitle = L"Select Folder with Animation Frames";
    LPITEMIDLIST pidl = SHBrowseForFolder(&bi);

    if (pidl != 0) {
        WCHAR folderPath[MAX_PATH];
        SHGetPathFromIDList(pidl, folderPath);
        SetWindowText(hwndEditFolder, folderPath);
        selectedFolderPath = folderPath;
        CoTaskMemFree(pidl);
    }
}


void PetManagerWindow::OnStart(HWND hWnd) {
    BOOL animationMode = SendMessage(hwndCheckAnimation, BM_GETCHECK, 0, 0);

    // Always stop existing pet first
    if (petHwnd) {
        DestroyWindow(petHwnd);
        petHwnd = nullptr;
    }

    if (animationMode) {
        if (selectedFolderPath.empty()) {
            MessageBox(hWnd, L"Select animation folder first.", L"Error", MB_OK);
            return;
        }
        petHwnd = CreatePetWindow(128, 128, selectedFolderPath, true);
    }
    else {
        if (selectedImagePath.empty()) {
            MessageBox(hWnd, L"Select a static image first.", L"Error", MB_OK);
            return;
        }
        petHwnd = CreatePetWindow(128, 128, selectedImagePath, false);
    }
}

void PetManagerWindow::OnStop(HWND) {
    if (petHwnd) {
        DestroyWindow(petHwnd);
        petHwnd = nullptr;
    }
}