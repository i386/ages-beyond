#pragma once

#ifndef AgesBeyondEvents_h
#define AgesBeyondEvents_h

class CvCity;

namespace AgesBeyond
{
	void OnGameStarted();
	void OnCityFounded(CvCity* pCity);
	void OnWarDeclared(TeamTypes eDeclaringTeam, TeamTypes eTargetTeam, WarPlanTypes eWarPlan);
	void OnPeaceSigned(TeamTypes eFirstTeam, TeamTypes eSecondTeam);
	void OnTechDiscovered(TeamTypes eTeam, PlayerTypes ePlayer, TechTypes eTech);
	void OnReligionFounded(ReligionTypes eReligion, CvCity* pHolyCity);
	void OnWonderBuilt(CvCity* pCity, BuildingTypes eBuilding);
}

#endif
