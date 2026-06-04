#include "CvGameCoreDLL.h"
#include "AgesBeyondCompanion.h"

#include <stdio.h>
#include <string.h>

namespace
{
	const int PROTOCOL_VERSION = 1;
	const DWORD CONNECT_TIMEOUT_MS = 5000;
	const DWORD CONNECT_POLL_MS = 100;
	const DWORD SHUTDOWN_TIMEOUT_MS = 2000;

	HANDLE g_hPipe = INVALID_HANDLE_VALUE;
	HANDLE g_hReaderThread = NULL;
	PROCESS_INFORMATION g_kProcessInfo;
	CRITICAL_SECTION g_kPipeSection;
	bool g_bPipeSectionInitialized = false;
	bool g_bStopReader = false;
	volatile LONG g_iNextRequestId = 1;

	void Trace(const char* szMessage)
	{
#ifndef FINAL_RELEASE
		OutputDebugStringA(szMessage);
		OutputDebugStringA("\n");
#endif
	}

	CvString GetParentDirectory(const CvString& szPath)
	{
		size_t iSlash = szPath.find_last_of("\\/");
		if (iSlash == CvString::npos)
		{
			return "";
		}
		return szPath.substr(0, iSlash);
	}

	bool FileExists(const CvString& szPath)
	{
		DWORD dwAttributes = GetFileAttributesA(szPath.c_str());
		return (dwAttributes != INVALID_FILE_ATTRIBUTES && (dwAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0);
	}

	bool GetDllDirectory(CvString& szDirectory)
	{
		char szModulePath[MAX_PATH];
		HMODULE hModule = GetModuleHandleA("CvGameCoreDLL.dll");
		DWORD dwLength = GetModuleFileNameA(hModule, szModulePath, MAX_PATH);
		if (dwLength == 0 || dwLength >= MAX_PATH)
		{
			return false;
		}

		szDirectory = GetParentDirectory(szModulePath);
		return !szDirectory.empty();
	}

	bool FindCompanionExe(CvString& szExePath)
	{
		CvString szDllDir;
		if (!GetDllDirectory(szDllDir))
		{
			return false;
		}

		CvString aszCandidates[] =
		{
			szDllDir + "\\AgesBeyondCompanion.exe",
			szDllDir + "\\..\\Companion\\AgesBeyondCompanion.exe",
			szDllDir + "\\..\\AgesBeyondCompanion.exe"
		};

		for (int iI = 0; iI < 3; ++iI)
		{
			if (FileExists(aszCandidates[iI]))
			{
				szExePath = aszCandidates[iI];
				return true;
			}
		}

		return false;
	}

	bool GetChroniclePath(CvString& szChroniclePath)
	{
		CvString szDllDir;
		if (!GetDllDirectory(szDllDir))
		{
			return false;
		}

		szChroniclePath = szDllDir + "\\..\\Chronicle\\AgesBeyondChronicle.md";
		return true;
	}

	CvString QuoteCommandArgument(const CvString& szArgument)
	{
		CvString szQuoted = "\"";
		for (int iI = 0; iI < (int)szArgument.length(); ++iI)
		{
			if (szArgument[iI] == '"')
			{
				szQuoted += "\\\"";
			}
			else
			{
				szQuoted += szArgument[iI];
			}
		}
		szQuoted += "\"";
		return szQuoted;
	}

	CvString JsonEscape(const char* szText)
	{
		CvString szEscaped;
		if (szText == NULL)
		{
			return szEscaped;
		}

		for (const unsigned char* p = (const unsigned char*)szText; *p != 0; ++p)
		{
			switch (*p)
			{
			case '\\':
				szEscaped += "\\\\";
				break;
			case '"':
				szEscaped += "\\\"";
				break;
			case '\b':
				szEscaped += "\\b";
				break;
			case '\f':
				szEscaped += "\\f";
				break;
			case '\n':
				szEscaped += "\\n";
				break;
			case '\r':
				szEscaped += "\\r";
				break;
			case '\t':
				szEscaped += "\\t";
				break;
			default:
				if (*p < 0x20)
				{
					char szBuffer[8];
					sprintf(szBuffer, "\\u%04x", *p);
					szEscaped += szBuffer;
				}
				else
				{
					szEscaped += (char)*p;
				}
				break;
			}
		}

		return szEscaped;
	}

	bool LaunchCompanionProcess(const CvString& szExePath, const CvString& szPipeName, const CvString& szChroniclePath)
	{
		CvString szCommandLine = QuoteCommandArgument(szExePath);
		szCommandLine += " --pipe ";
		szCommandLine += QuoteCommandArgument(szPipeName);
		szCommandLine += " --chronicle ";
		szCommandLine += QuoteCommandArgument(szChroniclePath);

		STARTUPINFOA kStartupInfo;
		ZeroMemory(&kStartupInfo, sizeof(kStartupInfo));
		kStartupInfo.cb = sizeof(kStartupInfo);
		kStartupInfo.dwFlags = STARTF_USESHOWWINDOW;
		kStartupInfo.wShowWindow = SW_HIDE;

		ZeroMemory(&g_kProcessInfo, sizeof(g_kProcessInfo));

		char* szMutableCommandLine = new char[szCommandLine.length() + 1];
		strcpy(szMutableCommandLine, szCommandLine.c_str());

		BOOL bStarted = CreateProcessA(
			szExePath.c_str(),
			szMutableCommandLine,
			NULL,
			NULL,
			FALSE,
			CREATE_NO_WINDOW,
			NULL,
			NULL,
			&kStartupInfo,
			&g_kProcessInfo);

		delete[] szMutableCommandLine;

		if (!bStarted)
		{
			Trace("Ages Beyond Companion: failed to launch companion process");
			return false;
		}

		return true;
	}

	bool ConnectPipe(const CvString& szPipeName)
	{
		DWORD dwStart = GetTickCount();
		while (GetTickCount() - dwStart < CONNECT_TIMEOUT_MS)
		{
			if (WaitNamedPipeA(szPipeName.c_str(), CONNECT_POLL_MS))
			{
				HANDLE hPipe = CreateFileA(
					szPipeName.c_str(),
					GENERIC_READ | GENERIC_WRITE,
					0,
					NULL,
					OPEN_EXISTING,
					0,
					NULL);

				if (hPipe != INVALID_HANDLE_VALUE)
				{
					DWORD dwMode = PIPE_READMODE_BYTE;
					SetNamedPipeHandleState(hPipe, &dwMode, NULL, NULL);
					g_hPipe = hPipe;
					return true;
				}
			}
			Sleep(CONNECT_POLL_MS);
		}

		Trace("Ages Beyond Companion: timed out waiting for named pipe");
		return false;
	}

	DWORD WINAPI ReaderThreadProc(LPVOID)
	{
		char szBuffer[512];

		while (!g_bStopReader)
		{
			DWORD dwBytesRead = 0;
			BOOL bRead = ReadFile(g_hPipe, szBuffer, sizeof(szBuffer) - 1, &dwBytesRead, NULL);
			if (!bRead || dwBytesRead == 0)
			{
				break;
			}

			szBuffer[dwBytesRead] = 0;
#ifndef FINAL_RELEASE
			OutputDebugStringA("Ages Beyond Companion response: ");
			OutputDebugStringA(szBuffer);
#endif
		}

		return 0;
	}

	void StartReaderThread()
	{
		g_bStopReader = false;
		g_hReaderThread = CreateThread(NULL, 0, ReaderThreadProc, NULL, 0, NULL);
	}

	bool WriteLine(const CvString& szLine)
	{
		if (g_hPipe == INVALID_HANDLE_VALUE)
		{
			return false;
		}

		CvString szOutput = szLine;
		szOutput += "\n";

		EnterCriticalSection(&g_kPipeSection);
		DWORD dwBytesWritten = 0;
		BOOL bWritten = WriteFile(g_hPipe, szOutput.c_str(), (DWORD)szOutput.length(), &dwBytesWritten, NULL);
		LeaveCriticalSection(&g_kPipeSection);

		return (bWritten && dwBytesWritten == szOutput.length());
	}

	CvString NextRequestId(const char* szPrefix)
	{
		long iRequest = InterlockedIncrement(&g_iNextRequestId);
		return CvString::format("%s-%ld", szPrefix, iRequest);
	}
}

namespace AgesBeyond
{
	void StartCompanion()
	{
		if (g_hPipe != INVALID_HANDLE_VALUE)
		{
			return;
		}

		CvString szExePath;
		if (!FindCompanionExe(szExePath))
		{
			Trace("Ages Beyond Companion: AgesBeyondCompanion.exe not found");
			return;
		}

		if (!g_bPipeSectionInitialized)
		{
			InitializeCriticalSection(&g_kPipeSection);
			g_bPipeSectionInitialized = true;
		}

		CvString szPipeName = CvString::format("\\\\.\\pipe\\AgesBeyond-%lu-%lu", GetCurrentProcessId(), GetTickCount());

		CvString szChroniclePath;
		if (!GetChroniclePath(szChroniclePath))
		{
			Trace("Ages Beyond Companion: failed to resolve chronicle path");
			return;
		}

		if (!LaunchCompanionProcess(szExePath, szPipeName, szChroniclePath))
		{
			return;
		}

		if (!ConnectPipe(szPipeName))
		{
			StopCompanion();
			return;
		}

		StartReaderThread();
		SendPing();
		Trace("Ages Beyond Companion: started");
	}

	void StopCompanion()
	{
		g_bStopReader = true;

		if (g_hPipe != INVALID_HANDLE_VALUE)
		{
			CloseHandle(g_hPipe);
			g_hPipe = INVALID_HANDLE_VALUE;
		}

		if (g_hReaderThread != NULL)
		{
			WaitForSingleObject(g_hReaderThread, SHUTDOWN_TIMEOUT_MS);
			CloseHandle(g_hReaderThread);
			g_hReaderThread = NULL;
		}

		if (g_kProcessInfo.hProcess != NULL)
		{
			if (WaitForSingleObject(g_kProcessInfo.hProcess, SHUTDOWN_TIMEOUT_MS) == WAIT_TIMEOUT)
			{
				TerminateProcess(g_kProcessInfo.hProcess, 0);
			}
			CloseHandle(g_kProcessInfo.hProcess);
			g_kProcessInfo.hProcess = NULL;
		}

		if (g_kProcessInfo.hThread != NULL)
		{
			CloseHandle(g_kProcessInfo.hThread);
			g_kProcessInfo.hThread = NULL;
		}

		if (g_bPipeSectionInitialized)
		{
			DeleteCriticalSection(&g_kPipeSection);
			g_bPipeSectionInitialized = false;
		}
	}

	bool IsCompanionRunning()
	{
		return g_hPipe != INVALID_HANDLE_VALUE;
	}

	bool SendPing()
	{
		CvString szRequest = CvString::format(
			"{\"version\":%d,\"id\":\"%s\",\"kind\":\"ping\"}",
			PROTOCOL_VERSION,
			NextRequestId("ping").c_str());
		return WriteLine(szRequest);
	}

	bool SendGameEvent(const char* szEventType, int iEventId, int iTurn, const char* szSummary, int iPlayer, int iTeam, int iCityId, int iX, int iY, int iData1, int iData2, const char* szFactsJson)
	{
		CvString szExtraFacts;
		if (szFactsJson != NULL && szFactsJson[0] != 0)
		{
			szExtraFacts = ",";
			szExtraFacts += szFactsJson;
		}

		CvString szRequest = CvString::format(
			"{\"version\":%d,\"id\":\"%s\",\"kind\":\"game_event\",\"event\":{\"event_type\":\"%s\",\"turn\":%d,\"actors\":[],\"summary\":\"%s\",\"facts\":{\"contract_version\":2,\"event_id\":%d,\"player_id\":%d,\"team_id\":%d,\"city_id\":%d,\"x\":%d,\"y\":%d,\"data1\":%d,\"data2\":%d,\"max_civ_players\":%d,\"barbarian_team_id\":%d%s}}}",
			PROTOCOL_VERSION,
			NextRequestId("event").c_str(),
			JsonEscape(szEventType).c_str(),
			iTurn,
			JsonEscape(szSummary).c_str(),
			iEventId,
			iPlayer,
			iTeam,
			iCityId,
			iX,
			iY,
			iData1,
			iData2,
			MAX_CIV_PLAYERS,
			BARBARIAN_TEAM,
			szExtraFacts.c_str());
		return WriteLine(szRequest);
	}
}
