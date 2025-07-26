#pragma once
#include <windows.h>

class PetManagerWindow {
public:
    void Init(HINSTANCE hInstance);
    void Show();

private:
    static LRESULT CALLBACK WndProc(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam);
    /*static void OnBrowse(HWND hDlg);
    static void OnStart(HWND hDlg);
    static void OnStop(HWND hDlg);*/

    void CreateControls(HWND hWnd);
    void OnBrowse(HWND hWnd);
    void OnStart(HWND hWnd);
    void OnStop(HWND hWnd);

    HWND hwndMain = nullptr;
    HWND hwndEdit = nullptr;
    HWND hwndBtnStart = nullptr;
    HWND hwndBtnStop = nullptr;
    HWND hwndBtnBrowse = nullptr;

    HINSTANCE hInst = nullptr;
};
