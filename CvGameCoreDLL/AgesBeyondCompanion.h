#pragma once

#ifndef AgesBeyondCompanion_h
#define AgesBeyondCompanion_h

namespace AgesBeyond
{
	void StartCompanion();
	void StopCompanion();
	bool IsCompanionRunning();
	bool SendPing();
	bool SendGameEvent(const char* szEventType, int iEventId, int iTurn, const char* szSummary, int iPlayer, int iTeam, int iCityId, int iX, int iY, int iData1, int iData2, const char* szFactsJson);
}

#endif
