
#define WIN32_LEAN_AND_MEAN
#include "shlobj_core.h"
#include "stdlib.h"
#include <string>
#include <locale>
#include <codecvt>
#include "windows.h"
#include <vector>
#include "winnls.h"
#include "shobjidl.h"
#include "objbase.h"
#include "objidl.h"
#include "shlguid.h"

using std::runtime_error;
using std::string;
using std::vector;
using std::wstring;

wstring utf8toUtf16(const string &str)
{
   if (str.empty())
      return wstring();

   size_t charsNeeded = ::MultiByteToWideChar(CP_UTF8, 0,
                                              str.data(), (int)str.size(), NULL, 0);
   if (charsNeeded == 0)
      throw runtime_error("Failed converting UTF-8 string to UTF-16");

   vector<wchar_t> buffer(charsNeeded);
   int charsConverted = ::MultiByteToWideChar(CP_UTF8, 0,
                                              str.data(), (int)str.size(), &buffer[0], buffer.size());
   if (charsConverted == 0)
      throw runtime_error("Failed converting UTF-8 string to UTF-16");

   return wstring(&buffer[0], charsConverted);
}
extern "C" int CreateLink(const char *target, const char *link_file_path, const char *link_icon_path, const char *args, const char *description, char admin)
{
   std::wstring wPathU16 = utf8toUtf16(target);
   std::wstring desU16 = utf8toUtf16(description);
   std::wstring iconpathU16 = utf8toUtf16(link_icon_path);
   std::wstring argsU16 = utf8toUtf16(args);
   LPCWSTR lpszPathObj = wPathU16.c_str();
   LPCSTR lpszPathLink = link_file_path;
   LPCWSTR lpszArgs = argsU16.c_str();
   LPCWSTR lpszLinkIcon = iconpathU16.c_str();
   LPCWSTR lpszDesc = desU16.c_str();
   HRESULT hres;
   IShellLink *psl;
   CoInitialize(NULL);
   // Get a pointer to the IShellLink interface. It is assumed that CoInitialize
   // has already been called.
   hres = CoCreateInstance(CLSID_ShellLink, NULL, CLSCTX_INPROC_SERVER, IID_IShellLink, (LPVOID *)&psl);
   if (SUCCEEDED(hres))
   {
      IPersistFile *ppf;

      // Set the path to the shortcut target and add the description.
      psl->SetPath(lpszPathObj);
      psl->SetDescription(lpszDesc);
      psl->SetIconLocation(lpszLinkIcon, 0);
      psl->SetArguments(lpszArgs);

      // Query IShellLink for the IPersistFile interface, used for saving the
      // shortcut in persistent storage.
      hres = psl->QueryInterface(IID_IPersistFile, (LPVOID *)&ppf);

      if (SUCCEEDED(hres))
      {
         WCHAR wsz[MAX_PATH];
         // Ensure that the string is Unicode.
         MultiByteToWideChar(CP_ACP, 0, lpszPathLink, -1, wsz, MAX_PATH);

         if (admin)
         {

            IShellLinkDataList *pdl;

            hres = psl->QueryInterface(IID_IShellLinkDataList, (void **)&pdl);
            if (SUCCEEDED(hres))
            {
               DWORD dwFlags = 0;
               hres = pdl->GetFlags(&dwFlags);
               if (SUCCEEDED(hres))
               {
                  hres = pdl->SetFlags(SLDF_RUNAS_USER | dwFlags);
                  if (SUCCEEDED(hres))
                  {
                     // Save the link by calling IPersistFile::Save.

                     hres = ppf->Save(wsz, TRUE);
                     if (SUCCEEDED(hres))
                     {
                        hres = ppf->SaveCompleted(wsz);
                     }
                  }
               }
            }
         }
         else
         {
            hres = ppf->Save(wsz, TRUE);
         }

         ppf->Release();
      }
      psl->Release();
   }
   CoUninitialize();
   return hres;
}
extern "C" int ScheduleFileDelete(const char *target)
{
   DWORD dwflags = MOVEFILE_DELAY_UNTIL_REBOOT;
   std::wstring targetUTF16 = utf8toUtf16(target);
   LPCWSTR targetptr = targetUTF16.c_str();
   BOOL res = MoveFileExW(targetptr, NULL, dwflags);
   if (res == 0)
   {
      DWORD lasterror = GetLastError();
      return lasterror;
   }
   return 0;
}
struct Window
{
   unsigned long pid;
   HWND hwnd;
};
static BOOL CALLBACK EnumWindowsCB(HWND hWnd, LPARAM lParam)
{
   Window &data = *(Window *)lParam;
   unsigned long process_id = 0;
   GetWindowThreadProcessId(hWnd, &process_id);
   if (data.pid != process_id)
      return TRUE;
   data.hwnd = hWnd;
   return FALSE;
}
static HWND WindowFromPID(unsigned long pid)
{
   Window w = {pid};
   if (!EnumWindows(EnumWindowsCB, (LPARAM)&w))
   {
      return w.hwnd;
   }
   return NULL;
}
// ok = 0, not found = 1, error = 2;
extern "C" int FocusPID(unsigned long pid)
{
   HWND hwnd = WindowFromPID(pid);
   if (hwnd == NULL)
   {
      // either not found or something is wrong
      if (GetLastError() == ERROR_SUCCESS)
      {
         // not found
         return 1;
      }
      else
      {
         return 2;
      }
   }
   // we now have the window handle

   if (!SetForegroundWindow(hwnd))
   {
      // error
      return 2;
   }
   else
   {

      return 0;
   }
}
