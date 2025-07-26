#pragma once
#include <windows.h>
#include <gdiplus.h>
using namespace Gdiplus;

HWND CreatePetWindow(const wchar_t* imagePath, int width, int height);