#include <windows.h>

#include <objidl.h>        // required by GDI+
#include <gdiplus.h>       // now safe to include
#pragma comment(lib, "Gdiplus.lib")
#include "PetManagerWindow.h"

using namespace Gdiplus;

// Global for GDI+

int APIENTRY wWinMain(_In_ HINSTANCE hInstance,
					  _In_opt_ HINSTANCE hPrevInstance,
					  _In_ LPWSTR    lpCmdLine,
					  _In_ int       nCmdShow)
{
	UNREFERENCED_PARAMETER(hPrevInstance);
	UNREFERENCED_PARAMETER(lpCmdLine);
	
	// Initialize GDI+
	GdiplusStartupInput gsi;
	ULONG_PTR gdiplusToken;
	GdiplusStartup(&gdiplusToken, &gsi, NULL);


	//MessageBox(NULL, L"wWinMain is running!", L"DEBUG", MB_OK);


	// Show the main Pet Manager UI Window
	
	PetManagerWindow manager;
	manager.Init(hInstance);
	manager.Show();

	// Standard Win32 message loop (this was missing!)
	MSG msg;
	while (GetMessage(&msg, nullptr, 0, 0)) {
		TranslateMessage(&msg);
		DispatchMessage(&msg);
	}

	GdiplusShutdown(gdiplusToken);
	return 0;
}