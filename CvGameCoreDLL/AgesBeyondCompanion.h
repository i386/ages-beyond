#pragma once

#ifndef AgesBeyondCompanion_h
#define AgesBeyondCompanion_h

namespace AgesBeyond
{
	void StartCompanion();
	void StopCompanion();
	bool IsCompanionRunning();
	bool SendPing();
	bool SendGameEvent(const char* szEventType, int iTurn, const char* szSummary);
}

#endif
