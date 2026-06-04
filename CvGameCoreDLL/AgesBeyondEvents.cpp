#include "CvGameCoreDLL.h"
#include "AgesBeyondEvents.h"
#include "CvCity.h"
#include "CvGame.h"
#include "CvPlayer.h"
#include "CvPlot.h"
#include "CvTeam.h"
#include "CvUnit.h"

namespace
{
	CvString Narrow(const wchar* szWide)
	{
		CvString szNarrow;
		szNarrow.Copy(szWide);
		return szNarrow;
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

	class JsonFacts
	{
	public:
		JsonFacts() : m_bFirst(true) {}

		void addString(const char* szKey, const char* szValue)
		{
			if (szValue == NULL || szValue[0] == 0)
			{
				return;
			}
			addComma();
			m_szFacts += "\"";
			m_szFacts += szKey;
			m_szFacts += "\":\"";
			m_szFacts += JsonEscape(szValue);
			m_szFacts += "\"";
		}

		void addWideString(const char* szKey, const wchar* szValue)
		{
			addString(szKey, Narrow(szValue).c_str());
		}

		void addInt(const char* szKey, int iValue)
		{
			addComma();
			m_szFacts += CvString::format("\"%s\":%d", szKey, iValue);
		}

		void addBool(const char* szKey, bool bValue)
		{
			addComma();
			m_szFacts += CvString::format("\"%s\":%s", szKey, bValue ? "true" : "false");
		}

		const CvString& str() const
		{
			return m_szFacts;
		}

	private:
		void addComma()
		{
			if (!m_bFirst)
			{
				m_szFacts += ",";
			}
			m_bFirst = false;
		}

		CvString m_szFacts;
		bool m_bFirst;
	};

	bool IsValidPlayer(PlayerTypes ePlayer)
	{
		return (ePlayer >= 0 && ePlayer < MAX_PLAYERS);
	}

	bool IsValidTeam(TeamTypes eTeam)
	{
		return (eTeam >= 0 && eTeam < MAX_TEAMS);
	}

	bool IsValidReligion(ReligionTypes eReligion)
	{
		return (eReligion >= 0 && eReligion < GC.getNumReligionInfos());
	}

	bool IsValidBuilding(BuildingTypes eBuilding)
	{
		return (eBuilding >= 0 && eBuilding < GC.getNumBuildingInfos());
	}

	bool IsValidProject(ProjectTypes eProject)
	{
		return (eProject >= 0 && eProject < GC.getNumProjectInfos());
	}

	bool IsValidTech(TechTypes eTech)
	{
		return (eTech >= 0 && eTech < GC.getNumTechInfos());
	}

	bool IsValidEvent(EventTypes eEvent)
	{
		return (eEvent >= 0 && eEvent < GC.getNumEventInfos());
	}

	bool IsNarrativeReady()
	{
		return GC.getGameINLINE().isFinalInitialized() && !gDLL->GetWorldBuilderMode();
	}

	PlayerTypes ActivePlayer()
	{
		return GC.getGameINLINE().getActivePlayer();
	}

	TeamTypes ActiveTeam()
	{
		return GC.getGameINLINE().getActiveTeam();
	}

	bool IsActivePlayer(PlayerTypes ePlayer)
	{
		return IsValidPlayer(ePlayer) && ePlayer == ActivePlayer();
	}

	bool IsActiveTeam(TeamTypes eTeam)
	{
		return IsValidTeam(eTeam) && eTeam == ActiveTeam();
	}

	bool ActiveTeamHasMet(TeamTypes eTeam)
	{
		TeamTypes eActiveTeam = ActiveTeam();
		return IsValidTeam(eActiveTeam) && IsValidTeam(eTeam) && (eTeam == eActiveTeam || GET_TEAM(eActiveTeam).isHasMet(eTeam));
	}

	bool ActiveTeamHasMetPlayer(PlayerTypes ePlayer)
	{
		if (!IsValidPlayer(ePlayer))
		{
			return false;
		}
		return ActiveTeamHasMet(GET_PLAYER(ePlayer).getTeam());
	}

	bool IsPlotVisibleToActiveTeam(CvPlot* pPlot)
	{
		TeamTypes eActiveTeam = ActiveTeam();
		return pPlot != NULL && IsValidTeam(eActiveTeam) && pPlot->isVisible(eActiveTeam, false);
	}

	bool IsPlotRevealedToActiveTeam(CvPlot* pPlot)
	{
		TeamTypes eActiveTeam = ActiveTeam();
		return pPlot != NULL && IsValidTeam(eActiveTeam) && pPlot->isRevealed(eActiveTeam, false);
	}

	bool IsPlotKnownToActiveTeam(CvPlot* pPlot)
	{
		return pPlot == NULL || IsPlotVisibleToActiveTeam(pPlot) || IsPlotRevealedToActiveTeam(pPlot) || GC.getGameINLINE().isDebugMode();
	}

	const char* PlotVisibility(CvPlot* pPlot)
	{
		if (pPlot == NULL)
		{
			return "none";
		}
		if (IsPlotVisibleToActiveTeam(pPlot) || GC.getGameINLINE().isDebugMode())
		{
			return "visible";
		}
		if (IsPlotRevealedToActiveTeam(pPlot))
		{
			return "revealed";
		}
		return "hidden";
	}

	CvPlot* PlotAt(int iX, int iY)
	{
		if (!GC.getMapINLINE().isPlot(iX, iY))
		{
			return NULL;
		}
		return GC.getMapINLINE().plotINLINE(iX, iY);
	}

	void AddAudienceFacts(JsonFacts& kFacts, const char* szScope, bool bInvolvesActivePlayer, bool bInvolvesActiveTeam, bool bGlobalAnnouncement, bool bKnownToActivePlayer, CvPlot* pPlot)
	{
		PlayerTypes eActivePlayer = ActivePlayer();
		TeamTypes eActiveTeam = ActiveTeam();
		bool bLocationKnown = IsPlotKnownToActiveTeam(pPlot);

		kFacts.addString("audience", "active_player");
		kFacts.addString("visibility_scope", szScope);
		kFacts.addInt("active_player_id", eActivePlayer);
		kFacts.addInt("active_team_id", eActiveTeam);
		kFacts.addBool("involves_active_player", bInvolvesActivePlayer);
		kFacts.addBool("involves_active_team", bInvolvesActiveTeam);
		kFacts.addBool("is_global_announcement", bGlobalAnnouncement);
		kFacts.addBool("known_to_active_player", bKnownToActivePlayer || GC.getGameINLINE().isDebugMode());
		kFacts.addString("plot_visibility", PlotVisibility(pPlot));
		kFacts.addBool("location_known_to_active_player", bLocationKnown);
	}

	void AddWorldFacts(JsonFacts& kFacts, const char* szImportance, const char* szChapter, const char* szArc)
	{
		EraTypes eEra = GC.getGameINLINE().getCurrentEra();
		kFacts.addString("importance", szImportance);
		kFacts.addString("chapter", szChapter);
		kFacts.addString("story_arc", szArc);
		kFacts.addInt("game_year", GC.getGameINLINE().getGameTurnYear());
		kFacts.addInt("era_id", eEra);
		if (eEra != NO_ERA)
		{
			kFacts.addString("era_type", GC.getEraInfo(eEra).getType());
			kFacts.addWideString("era_name", GC.getEraInfo(eEra).getDescription());
		}
	}

	void AddPlayerFacts(JsonFacts& kFacts, const char* szPrefix, PlayerTypes ePlayer)
	{
		if (!IsValidPlayer(ePlayer))
		{
			return;
		}

		CvPlayer& kPlayer = GET_PLAYER(ePlayer);
		kFacts.addInt(CvString::format("%s_id", szPrefix).c_str(), ePlayer);
		kFacts.addWideString(CvString::format("%s_name", szPrefix).c_str(), kPlayer.getName());
		kFacts.addWideString(CvString::format("%s_civilization", szPrefix).c_str(), kPlayer.getCivilizationDescription());
		kFacts.addWideString(CvString::format("%s_leader", szPrefix).c_str(), kPlayer.getName());
		kFacts.addInt(CvString::format("%s_team_id", szPrefix).c_str(), kPlayer.getTeam());
		kFacts.addInt(CvString::format("%s_city_count", szPrefix).c_str(), kPlayer.getNumCities());
		kFacts.addInt(CvString::format("%s_total_population", szPrefix).c_str(), kPlayer.getTotalPopulation());
		kFacts.addBool(CvString::format("%s_is_human", szPrefix).c_str(), kPlayer.isHuman());
		kFacts.addBool(CvString::format("%s_is_barbarian", szPrefix).c_str(), kPlayer.isBarbarian());
		kFacts.addBool(CvString::format("%s_is_minor", szPrefix).c_str(), kPlayer.isMinorCiv());
	}

	void AddTeamFacts(JsonFacts& kFacts, const char* szPrefix, TeamTypes eTeam)
	{
		if (!IsValidTeam(eTeam))
		{
			return;
		}

		CvTeam& kTeam = GET_TEAM(eTeam);
		kFacts.addInt(CvString::format("%s_id", szPrefix).c_str(), eTeam);
		kFacts.addWideString(CvString::format("%s_name", szPrefix).c_str(), kTeam.getName().GetCString());
		kFacts.addInt(CvString::format("%s_member_count", szPrefix).c_str(), kTeam.getNumMembers());
		kFacts.addBool(CvString::format("%s_is_barbarian", szPrefix).c_str(), kTeam.isBarbarian());
		kFacts.addBool(CvString::format("%s_is_minor", szPrefix).c_str(), kTeam.isMinorCiv());
		kFacts.addBool(CvString::format("%s_is_ever_alive", szPrefix).c_str(), kTeam.isEverAlive() != 0);

		PlayerTypes eLeader = kTeam.getLeaderID();
		if (IsValidPlayer(eLeader))
		{
			kFacts.addInt(CvString::format("%s_leader_player_id", szPrefix).c_str(), eLeader);
			kFacts.addWideString(CvString::format("%s_leader_name", szPrefix).c_str(), GET_PLAYER(eLeader).getName());
			kFacts.addWideString(CvString::format("%s_civilization", szPrefix).c_str(), GET_PLAYER(eLeader).getCivilizationDescription());
		}
	}

	void AddCityFacts(JsonFacts& kFacts, CvCity* pCity)
	{
		if (pCity == NULL)
		{
			return;
		}

		kFacts.addWideString("city_name", pCity->getName().GetCString());
		kFacts.addInt("city_population", pCity->getPopulation());
		kFacts.addInt("city_highest_population", pCity->getHighestPopulation());
		kFacts.addInt("city_religion_count", pCity->getReligionCount());
		kFacts.addInt("city_world_wonder_count", pCity->getNumWorldWonders());
		kFacts.addBool("city_is_capital", pCity->isCapital());
		kFacts.addBool("city_is_holy_city", pCity->isHolyCity());
		kFacts.addBool("city_is_coastal", pCity->isCoastal(GC.getMIN_WATER_SIZE_FOR_OCEAN()));
		kFacts.addInt("city_original_owner_id", pCity->getOriginalOwner());
		kFacts.addInt("city_previous_owner_id", pCity->getPreviousOwner());
	}

	int RecordEvent(const char* szEventType, const char* szSummary, PlayerTypes ePlayer, TeamTypes eTeam, int iCityId, int iX, int iY, int iData1, int iData2, const char* szFactsJson)
	{
		return GC.getGameINLINE().addAgesBeyondChronicleEvent(szEventType, szSummary, ePlayer, eTeam, iCityId, iX, iY, iData1, iData2, szFactsJson);
	}
}

namespace AgesBeyond
{
	void OnGameStarted()
	{
		if (!IsNarrativeReady())
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "major", "Genesis", "founding");
		kFacts.addInt("world_size_id", GC.getMapINLINE().getWorldSize());
		kFacts.addInt("land_plots", GC.getMapINLINE().getLandPlots());
		kFacts.addInt("alive_civilization_count", GC.getGameINLINE().countCivPlayersAlive());
		AddAudienceFacts(kFacts, "global", false, false, true, true, NULL);

		RecordEvent(
			"game_started",
			"A new game of Civilization IV: Ages Beyond has begun.",
			NO_PLAYER,
			NO_TEAM,
			-1,
			-1,
			-1,
			-1,
			-1,
			kFacts.str().c_str());
	}

	void OnCityFounded(CvCity* pCity)
	{
		if (!IsNarrativeReady() || pCity == NULL || !IsValidPlayer(pCity->getOwnerINLINE()))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, (GET_PLAYER(pCity->getOwnerINLINE()).getNumCities() <= 1) ? "epochal" : "major", "Foundations", "settlement");
		AddPlayerFacts(kFacts, "founder", pCity->getOwnerINLINE());
		AddCityFacts(kFacts, pCity);
		kFacts.addString("dynamic_quest_seed", "settlement_identity");
		kFacts.addString("quest_policy", "suggest_only");
		kFacts.addBool("rumor_possible", !IsActivePlayer(pCity->getOwnerINLINE()) && ActiveTeamHasMetPlayer(pCity->getOwnerINLINE()) && !IsPlotKnownToActiveTeam(pCity->plot()));
		kFacts.addString("rumor_channel", "travellers");
		AddAudienceFacts(
			kFacts,
			"plot_event",
			IsActivePlayer(pCity->getOwnerINLINE()),
			IsActiveTeam(pCity->getTeam()),
			false,
			IsActivePlayer(pCity->getOwnerINLINE()) || IsPlotKnownToActiveTeam(pCity->plot()),
			pCity->plot());

		RecordEvent(
			"city_founded",
			CvString::format("%s founded %s at (%d,%d).", Narrow(GET_PLAYER(pCity->getOwnerINLINE()).getCivilizationDescription()).c_str(), Narrow(pCity->getName().GetCString()).c_str(), pCity->getX_INLINE(), pCity->getY_INLINE()).c_str(),
			pCity->getOwnerINLINE(),
			pCity->getTeam(),
			pCity->getID(),
			pCity->getX_INLINE(),
			pCity->getY_INLINE(),
			-1,
			-1,
			kFacts.str().c_str());
	}

	void OnCityAcquired(PlayerTypes eOldOwner, PlayerTypes eNewOwner, CvCity* pCity, bool bConquest, bool bTrade)
	{
		if (!IsNarrativeReady() || pCity == NULL || !IsValidPlayer(eNewOwner))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, bConquest ? "major" : "minor", bConquest ? "Conquest" : "Diplomacy", bConquest ? "war" : "exchange");
		AddPlayerFacts(kFacts, "old_owner", eOldOwner);
		AddPlayerFacts(kFacts, "new_owner", eNewOwner);
		AddCityFacts(kFacts, pCity);
		kFacts.addBool("is_conquest", bConquest);
		kFacts.addBool("is_trade", bTrade);
		kFacts.addString("dynamic_quest_seed", bConquest ? "occupation_aftermath" : "city_transition");
		kFacts.addString("quest_policy", "suggest_only");
		kFacts.addBool("rumor_possible", !(IsActivePlayer(eOldOwner) || IsActivePlayer(eNewOwner)) && ((IsValidPlayer(eOldOwner) && ActiveTeamHasMetPlayer(eOldOwner)) || ActiveTeamHasMetPlayer(eNewOwner)) && !IsPlotKnownToActiveTeam(pCity->plot()));
		kFacts.addString("rumor_channel", bConquest ? "displaced witnesses" : "merchant reports");
		AddAudienceFacts(
			kFacts,
			"plot_event",
			IsActivePlayer(eOldOwner) || IsActivePlayer(eNewOwner),
			IsActiveTeam(IsValidPlayer(eOldOwner) ? GET_PLAYER(eOldOwner).getTeam() : NO_TEAM) || IsActiveTeam(GET_PLAYER(eNewOwner).getTeam()),
			false,
			IsActivePlayer(eOldOwner) || IsActivePlayer(eNewOwner) || IsPlotKnownToActiveTeam(pCity->plot()),
			pCity->plot());

		RecordEvent(
			bConquest ? "city_captured" : "city_acquired",
			CvString::format("%s came under the control of %s.", Narrow(pCity->getName().GetCString()).c_str(), Narrow(GET_PLAYER(eNewOwner).getCivilizationDescription()).c_str()).c_str(),
			eNewOwner,
			GET_PLAYER(eNewOwner).getTeam(),
			pCity->getID(),
			pCity->getX_INLINE(),
			pCity->getY_INLINE(),
			eOldOwner,
			bTrade ? 1 : 0,
			kFacts.str().c_str());
	}

	void OnCityRazed(CvCity* pCity, PlayerTypes eRazingPlayer)
	{
		if (!IsNarrativeReady() || pCity == NULL || !IsValidPlayer(eRazingPlayer))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "epochal", "Conquest", "fall");
		AddPlayerFacts(kFacts, "razing_player", eRazingPlayer);
		AddPlayerFacts(kFacts, "old_owner", pCity->getOwnerINLINE());
		AddCityFacts(kFacts, pCity);
		kFacts.addString("dynamic_quest_seed", "city_ruins_legacy");
		kFacts.addString("quest_policy", "suggest_only");
		kFacts.addBool("rumor_possible", !(IsActivePlayer(eRazingPlayer) || IsActivePlayer(pCity->getOwnerINLINE())) && (ActiveTeamHasMetPlayer(eRazingPlayer) || ActiveTeamHasMetPlayer(pCity->getOwnerINLINE())) && !IsPlotKnownToActiveTeam(pCity->plot()));
		kFacts.addString("rumor_channel", "refugee tales");
		AddAudienceFacts(
			kFacts,
			"plot_event",
			IsActivePlayer(eRazingPlayer) || IsActivePlayer(pCity->getOwnerINLINE()),
			IsActiveTeam(GET_PLAYER(eRazingPlayer).getTeam()) || IsActiveTeam(pCity->getTeam()),
			false,
			IsActivePlayer(eRazingPlayer) || IsActivePlayer(pCity->getOwnerINLINE()) || IsPlotKnownToActiveTeam(pCity->plot()),
			pCity->plot());

		RecordEvent(
			"city_razed",
			CvString::format("%s was razed by %s.", Narrow(pCity->getName().GetCString()).c_str(), Narrow(GET_PLAYER(eRazingPlayer).getCivilizationDescription()).c_str()).c_str(),
			eRazingPlayer,
			GET_PLAYER(eRazingPlayer).getTeam(),
			pCity->getID(),
			pCity->getX_INLINE(),
			pCity->getY_INLINE(),
			pCity->getOwnerINLINE(),
			-1,
			kFacts.str().c_str());
	}

	void OnWarDeclared(TeamTypes eDeclaringTeam, TeamTypes eTargetTeam, WarPlanTypes eWarPlan)
	{
		if (!IsNarrativeReady() || !IsValidTeam(eDeclaringTeam) || !IsValidTeam(eTargetTeam))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "major", "War and Peace", "diplomacy");
		AddTeamFacts(kFacts, "declaring_team", eDeclaringTeam);
		AddTeamFacts(kFacts, "target_team", eTargetTeam);
		kFacts.addInt("target_team_id", eTargetTeam);
		kFacts.addInt("war_plan_id", eWarPlan);
		kFacts.addString("dynamic_quest_seed", "war_aims");
		kFacts.addString("quest_policy", "suggest_only");
		AddAudienceFacts(
			kFacts,
			"diplomacy",
			false,
			IsActiveTeam(eDeclaringTeam) || IsActiveTeam(eTargetTeam),
			false,
			IsActiveTeam(eDeclaringTeam) || IsActiveTeam(eTargetTeam) || (ActiveTeamHasMet(eDeclaringTeam) && ActiveTeamHasMet(eTargetTeam)),
			NULL);

		RecordEvent(
			"war_declared",
			CvString::format("%s declared war on %s.", Narrow(GET_TEAM(eDeclaringTeam).getName().GetCString()).c_str(), Narrow(GET_TEAM(eTargetTeam).getName().GetCString()).c_str()).c_str(),
			NO_PLAYER,
			eDeclaringTeam,
			-1,
			-1,
			-1,
			eTargetTeam,
			eWarPlan,
			kFacts.str().c_str());
	}

	void OnPeaceSigned(TeamTypes eFirstTeam, TeamTypes eSecondTeam)
	{
		if (!IsNarrativeReady() || !IsValidTeam(eFirstTeam) || !IsValidTeam(eSecondTeam))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "major", "War and Peace", "diplomacy");
		AddTeamFacts(kFacts, "first_team", eFirstTeam);
		AddTeamFacts(kFacts, "second_team", eSecondTeam);
		kFacts.addInt("target_team_id", eSecondTeam);
		kFacts.addString("dynamic_quest_seed", "peace_terms");
		kFacts.addString("quest_policy", "suggest_only");
		AddAudienceFacts(
			kFacts,
			"diplomacy",
			false,
			IsActiveTeam(eFirstTeam) || IsActiveTeam(eSecondTeam),
			false,
			IsActiveTeam(eFirstTeam) || IsActiveTeam(eSecondTeam) || (ActiveTeamHasMet(eFirstTeam) && ActiveTeamHasMet(eSecondTeam)),
			NULL);

		RecordEvent(
			"peace_signed",
			CvString::format("%s made peace with %s.", Narrow(GET_TEAM(eFirstTeam).getName().GetCString()).c_str(), Narrow(GET_TEAM(eSecondTeam).getName().GetCString()).c_str()).c_str(),
			NO_PLAYER,
			eFirstTeam,
			-1,
			-1,
			-1,
			eSecondTeam,
			-1,
			kFacts.str().c_str());
	}

	void OnTechDiscovered(TeamTypes eTeam, PlayerTypes ePlayer, TechTypes eTech)
	{
		if (!IsNarrativeReady() || !IsValidTeam(eTeam) || !IsValidPlayer(ePlayer) || !IsValidTech(eTech))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "major", "Knowledge", "discovery");
		AddTeamFacts(kFacts, "team", eTeam);
		AddPlayerFacts(kFacts, "discoverer", ePlayer);
		kFacts.addString("tech_type", GC.getTechInfo(eTech).getType());
		kFacts.addWideString("tech_name", GC.getTechInfo(eTech).getDescription());
		kFacts.addInt("tech_era_id", GC.getTechInfo(eTech).getEra());
		if (GC.getTechInfo(eTech).getEra() != NO_ERA)
		{
			kFacts.addString("tech_era_type", GC.getEraInfo((EraTypes)GC.getTechInfo(eTech).getEra()).getType());
			kFacts.addWideString("tech_era_name", GC.getEraInfo((EraTypes)GC.getTechInfo(eTech).getEra()).getDescription());
		}
		kFacts.addString("dynamic_quest_seed", "new_knowledge");
		kFacts.addString("quest_policy", "suggest_only");
		AddAudienceFacts(
			kFacts,
			"team_progress",
			IsActivePlayer(ePlayer),
			IsActiveTeam(eTeam),
			false,
			IsActivePlayer(ePlayer) || IsActiveTeam(eTeam),
			NULL);

		RecordEvent(
			"tech_discovered",
			CvString::format("%s discovered %s.", Narrow(GET_TEAM(eTeam).getName().GetCString()).c_str(), Narrow(GC.getTechInfo(eTech).getDescription()).c_str()).c_str(),
			ePlayer,
			eTeam,
			-1,
			-1,
			-1,
			eTech,
			-1,
			kFacts.str().c_str());
	}

	void OnReligionFounded(ReligionTypes eReligion, CvCity* pHolyCity)
	{
		if (!IsNarrativeReady() || pHolyCity == NULL || !IsValidReligion(eReligion) || !IsValidPlayer(pHolyCity->getOwnerINLINE()))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "epochal", "Faith", "religion");
		AddPlayerFacts(kFacts, "founder", pHolyCity->getOwnerINLINE());
		AddCityFacts(kFacts, pHolyCity);
		kFacts.addString("religion_type", GC.getReligionInfo(eReligion).getType());
		kFacts.addWideString("religion_name", GC.getReligionInfo(eReligion).getDescription());
		kFacts.addString("dynamic_quest_seed", "holy_city");
		kFacts.addString("quest_policy", "suggest_only");
		AddAudienceFacts(
			kFacts,
			"global_announcement",
			IsActivePlayer(pHolyCity->getOwnerINLINE()),
			IsActiveTeam(pHolyCity->getTeam()),
			true,
			true,
			pHolyCity->plot());

		RecordEvent(
			"religion_founded",
			CvString::format("%s was founded in %s.", Narrow(GC.getReligionInfo(eReligion).getDescription()).c_str(), Narrow(pHolyCity->getName().GetCString()).c_str()).c_str(),
			pHolyCity->getOwnerINLINE(),
			pHolyCity->getTeam(),
			pHolyCity->getID(),
			pHolyCity->getX_INLINE(),
			pHolyCity->getY_INLINE(),
			eReligion,
			-1,
			kFacts.str().c_str());
	}

	void OnWonderBuilt(CvCity* pCity, BuildingTypes eBuilding)
	{
		if (!IsNarrativeReady() || pCity == NULL || !IsValidBuilding(eBuilding) || !IsValidPlayer(pCity->getOwnerINLINE()))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "epochal", "Works of Wonder", "achievement");
		AddPlayerFacts(kFacts, "builder", pCity->getOwnerINLINE());
		AddCityFacts(kFacts, pCity);
		kFacts.addString("building_type", GC.getBuildingInfo(eBuilding).getType());
		kFacts.addWideString("building_name", GC.getBuildingInfo(eBuilding).getDescription());
		kFacts.addString("dynamic_quest_seed", "wonder_legacy");
		kFacts.addString("quest_policy", "suggest_only");
		AddAudienceFacts(
			kFacts,
			"global_announcement",
			IsActivePlayer(pCity->getOwnerINLINE()),
			IsActiveTeam(pCity->getTeam()),
			true,
			true,
			pCity->plot());

		RecordEvent(
			"wonder_built",
			CvString::format("%s completed %s in %s.", Narrow(GET_PLAYER(pCity->getOwnerINLINE()).getCivilizationDescription()).c_str(), Narrow(GC.getBuildingInfo(eBuilding).getDescription()).c_str(), Narrow(pCity->getName().GetCString()).c_str()).c_str(),
			pCity->getOwnerINLINE(),
			pCity->getTeam(),
			pCity->getID(),
			pCity->getX_INLINE(),
			pCity->getY_INLINE(),
			eBuilding,
			-1,
			kFacts.str().c_str());
	}

	void OnProjectBuilt(CvCity* pCity, ProjectTypes eProject)
	{
		if (!IsNarrativeReady() || pCity == NULL || !IsValidProject(eProject) || !IsValidPlayer(pCity->getOwnerINLINE()))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, GC.getProjectInfo(eProject).isSpaceship() ? "epochal" : "major", "Great Projects", "achievement");
		AddPlayerFacts(kFacts, "builder", pCity->getOwnerINLINE());
		AddTeamFacts(kFacts, "team", pCity->getTeam());
		AddCityFacts(kFacts, pCity);
		kFacts.addString("project_type", GC.getProjectInfo(eProject).getType());
		kFacts.addWideString("project_name", GC.getProjectInfo(eProject).getDescription());
		kFacts.addBool("project_is_spaceship", GC.getProjectInfo(eProject).isSpaceship());
		kFacts.addInt("project_victory_prereq_id", GC.getProjectInfo(eProject).getVictoryPrereq());
		kFacts.addString("dynamic_quest_seed", "great_project_consequences");
		kFacts.addString("quest_policy", "suggest_only");
		AddAudienceFacts(
			kFacts,
			"global_announcement",
			IsActivePlayer(pCity->getOwnerINLINE()),
			IsActiveTeam(pCity->getTeam()),
			true,
			true,
			pCity->plot());

		RecordEvent(
			"project_built",
			CvString::format("%s completed %s in %s.", Narrow(GET_PLAYER(pCity->getOwnerINLINE()).getCivilizationDescription()).c_str(), Narrow(GC.getProjectInfo(eProject).getDescription()).c_str(), Narrow(pCity->getName().GetCString()).c_str()).c_str(),
			pCity->getOwnerINLINE(),
			pCity->getTeam(),
			pCity->getID(),
			pCity->getX_INLINE(),
			pCity->getY_INLINE(),
			eProject,
			GC.getProjectInfo(eProject).getVictoryPrereq(),
			kFacts.str().c_str());
	}

	void OnGoldenAgeStarted(PlayerTypes ePlayer)
	{
		if (!IsNarrativeReady() || !IsValidPlayer(ePlayer))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "epochal", "Golden Ages", "flourishing");
		AddPlayerFacts(kFacts, "player", ePlayer);
		kFacts.addString("dynamic_quest_seed", "golden_age_mandate");
		kFacts.addString("quest_policy", "suggest_only");
		AddAudienceFacts(
			kFacts,
			"society",
			IsActivePlayer(ePlayer),
			IsActiveTeam(GET_PLAYER(ePlayer).getTeam()),
			false,
			IsActivePlayer(ePlayer) || ActiveTeamHasMetPlayer(ePlayer),
			NULL);

		RecordEvent(
			"golden_age_started",
			CvString::format("%s entered a golden age.", Narrow(GET_PLAYER(ePlayer).getCivilizationDescription()).c_str()).c_str(),
			ePlayer,
			GET_PLAYER(ePlayer).getTeam(),
			-1,
			-1,
			-1,
			GET_PLAYER(ePlayer).getGoldenAgeTurns(),
			-1,
			kFacts.str().c_str());
	}

	void OnGreatPersonBorn(CvUnit* pGreatPerson, PlayerTypes ePlayer, CvCity* pCity, int iX, int iY)
	{
		if (!IsNarrativeReady() || pGreatPerson == NULL || !IsValidPlayer(ePlayer))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "major", "Great People", "personage");
		AddPlayerFacts(kFacts, "player", ePlayer);
		AddCityFacts(kFacts, pCity);
		kFacts.addWideString("unit_name", pGreatPerson->getName().GetCString());
		kFacts.addString("unit_type", GC.getUnitInfo(pGreatPerson->getUnitType()).getType());
		kFacts.addWideString("unit_type_name", GC.getUnitInfo(pGreatPerson->getUnitType()).getDescription());
		kFacts.addString("dynamic_quest_seed", "great_person_legacy");
		kFacts.addString("quest_policy", "suggest_only");
		AddAudienceFacts(
			kFacts,
			"plot_event",
			IsActivePlayer(ePlayer),
			IsActiveTeam(GET_PLAYER(ePlayer).getTeam()),
			false,
			IsActivePlayer(ePlayer) || IsPlotKnownToActiveTeam((pCity != NULL) ? pCity->plot() : PlotAt(iX, iY)),
			(pCity != NULL) ? pCity->plot() : PlotAt(iX, iY));

		RecordEvent(
			"great_person_born",
			CvString::format("%s was born to %s.", Narrow(pGreatPerson->getName().GetCString()).c_str(), Narrow(GET_PLAYER(ePlayer).getCivilizationDescription()).c_str()).c_str(),
			ePlayer,
			GET_PLAYER(ePlayer).getTeam(),
			(pCity != NULL) ? pCity->getID() : -1,
			iX,
			iY,
			pGreatPerson->getUnitType(),
			-1,
			kFacts.str().c_str());
	}

	void OnQuestStarted(PlayerTypes ePlayer, EventTypes eEvent)
	{
		if (!IsNarrativeReady() || !IsValidPlayer(ePlayer) || !IsValidEvent(eEvent))
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "major", "Quests and Portents", "quest");
		AddPlayerFacts(kFacts, "player", ePlayer);
		kFacts.addString("event_type_key", GC.getEventInfo(eEvent).getType());
		kFacts.addWideString("event_name", GC.getEventInfo(eEvent).getDescription());
		kFacts.addBool("event_is_quest", GC.getEventInfo(eEvent).isQuest());
		kFacts.addString("quest_policy", "observe_existing_game_quest");
		AddAudienceFacts(
			kFacts,
			"player_event",
			IsActivePlayer(ePlayer),
			IsActiveTeam(GET_PLAYER(ePlayer).getTeam()),
			false,
			IsActivePlayer(ePlayer),
			NULL);

		RecordEvent(
			GC.getEventInfo(eEvent).isQuest() ? "quest_started" : "event_triggered",
			CvString::format("%s received %s.", Narrow(GET_PLAYER(ePlayer).getCivilizationDescription()).c_str(), Narrow(GC.getEventInfo(eEvent).getDescription()).c_str()).c_str(),
			ePlayer,
			GET_PLAYER(ePlayer).getTeam(),
			-1,
			-1,
			-1,
			eEvent,
			-1,
			kFacts.str().c_str());
	}

	void OnVictory(TeamTypes eTeam, VictoryTypes eVictory)
	{
		if (!IsNarrativeReady() || !IsValidTeam(eTeam) || eVictory == NO_VICTORY)
		{
			return;
		}

		JsonFacts kFacts;
		AddWorldFacts(kFacts, "epochal", "Final Age", "victory");
		AddTeamFacts(kFacts, "winner_team", eTeam);
		kFacts.addString("victory_type", GC.getVictoryInfo(eVictory).getType());
		kFacts.addWideString("victory_name", GC.getVictoryInfo(eVictory).getDescription());
		kFacts.addString("quest_policy", "finale");
		AddAudienceFacts(
			kFacts,
			"global_announcement",
			IsActivePlayer(GET_TEAM(eTeam).getLeaderID()),
			IsActiveTeam(eTeam),
			true,
			true,
			NULL);

		RecordEvent(
			"victory",
			CvString::format("%s achieved %s.", Narrow(GET_TEAM(eTeam).getName().GetCString()).c_str(), Narrow(GC.getVictoryInfo(eVictory).getDescription()).c_str()).c_str(),
			GET_TEAM(eTeam).getLeaderID(),
			eTeam,
			-1,
			-1,
			-1,
			eVictory,
			-1,
			kFacts.str().c_str());
	}
}
