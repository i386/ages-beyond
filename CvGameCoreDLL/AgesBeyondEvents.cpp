#include "CvGameCoreDLL.h"
#include "AgesBeyondEvents.h"
#include "CvCity.h"
#include "CvGame.h"

namespace
{
	int RecordEvent(const char* szEventType, const char* szSummary, PlayerTypes ePlayer, TeamTypes eTeam, int iCityId, int iX, int iY, int iData1, int iData2)
	{
		return GC.getGameINLINE().addAgesBeyondChronicleEvent(szEventType, szSummary, ePlayer, eTeam, iCityId, iX, iY, iData1, iData2);
	}
}

namespace AgesBeyond
{
	void OnGameStarted()
	{
		RecordEvent(
			"game_started",
			"A new game of Civilization IV: Ages Beyond has begun.",
			NO_PLAYER,
			NO_TEAM,
			-1,
			-1,
			-1,
			-1,
			-1);
	}

	void OnCityFounded(CvCity* pCity)
	{
		if (pCity == NULL)
		{
			return;
		}

		RecordEvent(
			"city_founded",
			CvString::format("Player %d founded city id %d at (%d,%d).", pCity->getOwnerINLINE(), pCity->getID(), pCity->getX_INLINE(), pCity->getY_INLINE()).c_str(),
			pCity->getOwnerINLINE(),
			pCity->getTeam(),
			pCity->getID(),
			pCity->getX_INLINE(),
			pCity->getY_INLINE(),
			-1,
			-1);
	}

	void OnWarDeclared(TeamTypes eDeclaringTeam, TeamTypes eTargetTeam, WarPlanTypes eWarPlan)
	{
		RecordEvent(
			"war_declared",
			CvString::format("Team %d declared war on team %d.", eDeclaringTeam, eTargetTeam).c_str(),
			NO_PLAYER,
			eDeclaringTeam,
			-1,
			-1,
			-1,
			eTargetTeam,
			eWarPlan);
	}

	void OnPeaceSigned(TeamTypes eFirstTeam, TeamTypes eSecondTeam)
	{
		RecordEvent(
			"peace_signed",
			CvString::format("Team %d made peace with team %d.", eFirstTeam, eSecondTeam).c_str(),
			NO_PLAYER,
			eFirstTeam,
			-1,
			-1,
			-1,
			eSecondTeam,
			-1);
	}

	void OnTechDiscovered(TeamTypes eTeam, PlayerTypes ePlayer, TechTypes eTech)
	{
		RecordEvent(
			"tech_discovered",
			CvString::format("Team %d discovered tech id %d.", eTeam, eTech).c_str(),
			ePlayer,
			eTeam,
			-1,
			-1,
			-1,
			eTech,
			-1);
	}

	void OnReligionFounded(ReligionTypes eReligion, CvCity* pHolyCity)
	{
		if (pHolyCity == NULL)
		{
			return;
		}

		RecordEvent(
			"religion_founded",
			CvString::format("Religion id %d was founded in city id %d at (%d,%d).", eReligion, pHolyCity->getID(), pHolyCity->getX_INLINE(), pHolyCity->getY_INLINE()).c_str(),
			pHolyCity->getOwnerINLINE(),
			pHolyCity->getTeam(),
			pHolyCity->getID(),
			pHolyCity->getX_INLINE(),
			pHolyCity->getY_INLINE(),
			eReligion,
			-1);
	}

	void OnWonderBuilt(CvCity* pCity, BuildingTypes eBuilding)
	{
		if (pCity == NULL)
		{
			return;
		}

		RecordEvent(
			"wonder_built",
			CvString::format("Player %d completed world wonder building id %d in city id %d at (%d,%d).", pCity->getOwnerINLINE(), eBuilding, pCity->getID(), pCity->getX_INLINE(), pCity->getY_INLINE()).c_str(),
			pCity->getOwnerINLINE(),
			pCity->getTeam(),
			pCity->getID(),
			pCity->getX_INLINE(),
			pCity->getY_INLINE(),
			eBuilding,
			-1);
	}
}
