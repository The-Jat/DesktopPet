#pragma once
#include <windows.h>

class PetManagerWindow {
public:
    void Init(HINSTANCE hInstance);
    void Show();

private:
    static LRESULT CALLBACK WndProc(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam);

    void CreateControls(HWND hWnd);
    void OnBrowseImage(HWND hWnd);
    void OnBrowseFolder(HWND hWnd);
    void OnStart(HWND hWnd);
    void OnStop(HWND hWnd);

    HINSTANCE hInst = nullptr;
    HWND hwndMain = nullptr;

    // UI Controls
    HWND hwndCheckAnimation = nullptr;
    HWND hwndEditImage = nullptr;
    HWND hwndBtnBrowseImage = nullptr;
    HWND hwndEditFolder = nullptr;
    HWND hwndBtnBrowseFolder = nullptr;
    HWND hwndBtnStart = nullptr;
    HWND hwndBtnStop = nullptr;
};
