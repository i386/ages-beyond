#pragma once

#ifndef AgesBeyondEvents_h
#define AgesBeyondEvents_h

class CvCity;
class CvUnit;

namespace AgesBeyond
{
	void OnGameStarted();
	void OnCityFounded(CvCity* pCity);
	void OnCityAcquired(PlayerTypes eOldOwner, PlayerTypes eNewOwner, CvCity* pCity, bool bConquest, bool bTrade);
	void OnCityRazed(CvCity* pCity, PlayerTypes eRazingPlayer);
	void OnWarDeclared(TeamTypes eDeclaringTeam, TeamTypes eTargetTeam, WarPlanTypes eWarPlan);
	void OnPeaceSigned(TeamTypes eFirstTeam, TeamTypes eSecondTeam);
	void OnTechDiscovered(TeamTypes eTeam, PlayerTypes ePlayer, TechTypes eTech);
	void OnReligionFounded(ReligionTypes eReligion, CvCity* pHolyCity);
	void OnWonderBuilt(CvCity* pCity, BuildingTypes eBuilding);
	void OnProjectBuilt(CvCity* pCity, ProjectTypes eProject);
	void OnGoldenAgeStarted(PlayerTypes ePlayer);
	void OnGreatPersonBorn(CvUnit* pGreatPerson, PlayerTypes ePlayer, CvCity* pCity, int iX, int iY);
	void OnQuestStarted(PlayerTypes ePlayer, EventTypes eEvent);
	void OnVictory(TeamTypes eTeam, VictoryTypes eVictory);
}

#endif
