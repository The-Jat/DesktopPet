#pragma once

#include <windows.h>
#include <gdiplus.h>
#include <string>

// Initializes GDI+ (call this once in your main function before using anything GDI+ related)
void InitGDIPlus();

// Shutdown GDI+ (call this once at application exit)
void ShutdownGDIPlus();

// Creates a window with an animated pet (loads frames from directory)
HWND CreatePetWindow(int width, int height, const std::wstring& path, bool isAnimation);

// Creates a window with a static image pet
HWND CreateStaticPetWindow(const wchar_t* imagePath, int width, int height);

// Optional: For redrawing or forcing updates manually
void DrawPet(HWND hwnd);
