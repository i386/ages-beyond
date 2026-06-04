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
	const DWORD RESPONSE_TIMEOUT_MS = 120;

	HANDLE g_hPipe = INVALID_HANDLE_VALUE;
	HANDLE g_hReaderThread = NULL;
	HANDLE g_hResponseEvent = NULL;
	PROCESS_INFORMATION g_kProcessInfo;
	CRITICAL_SECTION g_kPipeSection;
	CRITICAL_SECTION g_kResponseSection;
	bool g_bPipeSectionInitialized = false;
	bool g_bResponseSectionInitialized = false;
	bool g_bStopReader = false;
	volatile LONG g_iNextRequestId = 1;

	struct PipeResponse
	{
		CvString m_szId;
		CvString m_szText;
		bool m_bOk;
	};

	std::vector<PipeResponse> g_aResponses;

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

	CvString JsonUnescape(const CvString& szText)
	{
		CvString szOutput;
		for (int iI = 0; iI < (int)szText.length(); ++iI)
		{
			if (szText[iI] == '\\' && iI + 1 < (int)szText.length())
			{
				++iI;
				switch (szText[iI])
				{
				case 'n':
					szOutput += '\n';
					break;
				case 'r':
					szOutput += '\r';
					break;
				case 't':
					szOutput += '\t';
					break;
				case '"':
				case '\\':
				case '/':
					szOutput += szText[iI];
					break;
				default:
					szOutput += szText[iI];
					break;
				}
			}
			else
			{
				szOutput += szText[iI];
			}
		}
		return szOutput;
	}

	bool ExtractJsonString(const CvString& szJson, const char* szKey, CvString& szValue)
	{
		CvString szNeedle = CvString::format("\"%s\"", szKey);
		size_t iKey = szJson.find(szNeedle);
		if (iKey == CvString::npos)
		{
			return false;
		}

		size_t iColon = szJson.find(':', iKey + szNeedle.length());
		if (iColon == CvString::npos)
		{
			return false;
		}

		size_t iStart = szJson.find('"', iColon + 1);
		if (iStart == CvString::npos)
		{
			return false;
		}

		CvString szRaw;
		bool bEscaped = false;
		const char* szJsonText = szJson.GetCString();
		for (size_t iI = iStart + 1; iI < szJson.length(); ++iI)
		{
			char ch = szJsonText[iI];
			if (bEscaped)
			{
				szRaw += '\\';
				szRaw += ch;
				bEscaped = false;
			}
			else if (ch == '\\')
			{
				bEscaped = true;
			}
			else if (ch == '"')
			{
				szValue = JsonUnescape(szRaw);
				return true;
			}
			else
			{
				szRaw += ch;
			}
		}

		return false;
	}

	void StoreResponseLine(const CvString& szLine)
	{
		CvString szId;
		if (!ExtractJsonString(szLine, "id", szId))
		{
			return;
		}
		if (szId.find("diplo-") != 0)
		{
			return;
		}

		CvString szStatus;
		ExtractJsonString(szLine, "status", szStatus);

		PipeResponse kResponse;
		kResponse.m_szId = szId;
		kResponse.m_bOk = (szStatus == "ok");
		ExtractJsonString(szLine, "text", kResponse.m_szText);

		EnterCriticalSection(&g_kResponseSection);
		g_aResponses.push_back(kResponse);
		LeaveCriticalSection(&g_kResponseSection);

		if (g_hResponseEvent != NULL)
		{
			SetEvent(g_hResponseEvent);
		}
	}

	bool TakeResponse(const CvString& szId, CvString& szText)
	{
		bool bFound = false;
		EnterCriticalSection(&g_kResponseSection);
		for (std::vector<PipeResponse>::iterator it = g_aResponses.begin(); it != g_aResponses.end(); ++it)
		{
			if (it->m_szId == szId)
			{
				if (it->m_bOk)
				{
					szText = it->m_szText;
				}
				g_aResponses.erase(it);
				bFound = true;
				break;
			}
		}
		LeaveCriticalSection(&g_kResponseSection);
		return bFound;
	}

	bool WaitForResponse(const CvString& szId, DWORD dwTimeoutMs, CvString& szText)
	{
		DWORD dwStart = GetTickCount();
		while (GetTickCount() - dwStart < dwTimeoutMs)
		{
			if (TakeResponse(szId, szText))
			{
				return true;
			}

			DWORD dwRemaining = dwTimeoutMs - (GetTickCount() - dwStart);
			WaitForSingleObject(g_hResponseEvent, (dwRemaining < 20) ? dwRemaining : 20);
		}

		return TakeResponse(szId, szText);
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
		CvString szPending;

		while (!g_bStopReader)
		{
			DWORD dwBytesRead = 0;
			BOOL bRead = ReadFile(g_hPipe, szBuffer, sizeof(szBuffer) - 1, &dwBytesRead, NULL);
			if (!bRead || dwBytesRead == 0)
			{
				break;
			}

			szBuffer[dwBytesRead] = 0;
			szPending += szBuffer;
			for (;;)
			{
				size_t iLineEnd = szPending.find('\n');
				if (iLineEnd == CvString::npos)
				{
					break;
				}

				CvString szLine = szPending.substr(0, iLineEnd);
				szPending = szPending.substr(iLineEnd + 1);
				StoreResponseLine(szLine);
#ifndef FINAL_RELEASE
				OutputDebugStringA("Ages Beyond Companion response: ");
				OutputDebugStringA(szLine.c_str());
				OutputDebugStringA("\n");
#endif
			}
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
		if (!g_bResponseSectionInitialized)
		{
			InitializeCriticalSection(&g_kResponseSection);
			g_bResponseSectionInitialized = true;
		}
		if (g_hResponseEvent == NULL)
		{
			g_hResponseEvent = CreateEvent(NULL, FALSE, FALSE, NULL);
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
		if (g_bResponseSectionInitialized)
		{
			DeleteCriticalSection(&g_kResponseSection);
			g_bResponseSectionInitialized = false;
		}
		if (g_hResponseEvent != NULL)
		{
			CloseHandle(g_hResponseEvent);
			g_hResponseEvent = NULL;
		}
		g_aResponses.clear();
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
			"{\"version\":%d,\"id\":\"%s\",\"kind\":\"game_event\",\"event\":{\"event_type\":\"%s\",\"turn\":%d,\"actors\":[],\"summary\":\"%s\",\"facts\":{\"contract_version\":3,\"event_id\":%d,\"player_id\":%d,\"team_id\":%d,\"city_id\":%d,\"x\":%d,\"y\":%d,\"data1\":%d,\"data2\":%d,\"max_civ_players\":%d,\"barbarian_team_id\":%d%s}}}",
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

	CvString RequestDiplomacyText(const char* szCommentType, int iActivePlayer, int iLeaderPlayer, int iTurn, const char* szActivePlayerName, const char* szActiveCivilization, const char* szLeaderName, const char* szLeaderCivilization, const char* szAttitude, bool bAtWar, const char* szPowerRelation, const char* szFallbackText)
	{
		if (g_hPipe == INVALID_HANDLE_VALUE)
		{
			return "";
		}

		CvString szId = NextRequestId("diplo");
		CvString szRequest = CvString::format(
			"{\"version\":%d,\"id\":\"%s\",\"kind\":\"diplomacy_text\",\"request\":{\"comment_type\":\"%s\",\"active_player_id\":%d,\"leader_player_id\":%d,\"turn\":%d,\"active_player_name\":\"%s\",\"active_civilization\":\"%s\",\"leader_name\":\"%s\",\"leader_civilization\":\"%s\",\"attitude\":\"%s\",\"at_war\":%s,\"power_relation\":\"%s\",\"fallback_text\":\"%s\"}}",
			PROTOCOL_VERSION,
			szId.c_str(),
			JsonEscape(szCommentType).c_str(),
			iActivePlayer,
			iLeaderPlayer,
			iTurn,
			JsonEscape(szActivePlayerName).c_str(),
			JsonEscape(szActiveCivilization).c_str(),
			JsonEscape(szLeaderName).c_str(),
			JsonEscape(szLeaderCivilization).c_str(),
			JsonEscape(szAttitude).c_str(),
			bAtWar ? "true" : "false",
			JsonEscape(szPowerRelation).c_str(),
			JsonEscape(szFallbackText).c_str());

		if (!WriteLine(szRequest))
		{
			return "";
		}

		CvString szResponse;
		if (!WaitForResponse(szId, RESPONSE_TIMEOUT_MS, szResponse))
		{
			return "";
		}

		return szResponse;
	}
}
