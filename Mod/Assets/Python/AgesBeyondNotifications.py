from CvPythonExtensions import *
import os

gc = CyGlobalContext()

_notificationOffset = 0
_notificationPath = None
_questNotificationOffset = 0
_questNotificationPath = None
_questDecisionOffset = 0
_questDecisionPath = None
_questDecisionResponsePath = None
_questJournalPath = None
_questJournalSignature = None
_pendingDecisionPopups = {}
_nextDecisionPopupId = 1
_madeQuestDecisions = {}
_lastProbeMessage = None
_missingNotificationChannels = {}
_shownCount = 0


def reset():
	global _notificationOffset
	global _questNotificationOffset
	global _questDecisionOffset
	global _questJournalSignature
	global _pendingDecisionPopups
	global _nextDecisionPopupId
	global _madeQuestDecisions
	global _missingNotificationChannels
	_notificationOffset = 0
	_questNotificationOffset = 0
	_questDecisionOffset = 0
	_questJournalSignature = None
	_pendingDecisionPopups = {}
	_nextDecisionPopupId = 1
	_madeQuestDecisions = {}
	_missingNotificationChannels = {}
	writeProbe("reset")


def poll(iUpdate=0):
	if iUpdate % 5 != 0:
		return

	pollChannel("chronicle", getNotificationPath, "_notificationOffset", False)
	pollChannel("quest", getQuestNotificationPath, "_questNotificationOffset", True)
	pollQuestDecisions()
	pollQuestJournal()


def pollChannel(channel, pathGetter, offsetName, showPrefix):
	global _notificationOffset
	global _questNotificationOffset

	offset = globals()[offsetName]
	path = pathGetter()
	if not path or not os.path.exists(path):
		if not _missingNotificationChannels.get(channel, False):
			_missingNotificationChannels[channel] = True
			writeProbe("missing %s notification file" % channel)
		return
	_missingNotificationChannels[channel] = False

	try:
		size = os.path.getsize(path)
		if size < offset:
			offset = 0

		file = open(path, "rb")
		try:
			file.seek(offset)
			while True:
				lineStart = file.tell()
				line = file.readline()
				if not line:
					break
				if showLine(line, showPrefix):
					offset = file.tell()
				else:
					file.seek(lineStart)
					break
		finally:
			file.close()
		globals()[offsetName] = offset
	except Exception, e:
		writeProbe("%s poll failed: %s" % (channel, e))
		print "AgesBeyondNotifications: failed to poll %s notification file" % channel


def getNotificationPath():
	global _notificationPath

	if _notificationPath is not None and os.path.exists(_notificationPath):
		return _notificationPath

	for path in getCandidatePaths():
		if os.path.exists(path):
			_notificationPath = path
			writeProbe("using notification file: %s" % path)
			return _notificationPath

	if _notificationPath is None:
		candidates = getCandidatePaths()
		if candidates:
			_notificationPath = candidates[0]

	return _notificationPath


def getQuestNotificationPath():
	global _questNotificationPath

	if _questNotificationPath is not None and os.path.exists(_questNotificationPath):
		return _questNotificationPath

	for path in getQuestCandidatePaths():
		if os.path.exists(path):
			_questNotificationPath = path
			writeProbe("using quest notification file: %s" % path)
			return _questNotificationPath

	if _questNotificationPath is None:
		candidates = getQuestCandidatePaths()
		if candidates:
			_questNotificationPath = candidates[0]

	return _questNotificationPath


def getQuestDecisionPath():
	global _questDecisionPath

	if _questDecisionPath is not None and os.path.exists(_questDecisionPath):
		return _questDecisionPath

	for path in getQuestDecisionCandidatePaths():
		if os.path.exists(path):
			_questDecisionPath = path
			writeProbe("using quest decision file: %s" % path)
			return _questDecisionPath

	if _questDecisionPath is None:
		candidates = getQuestDecisionCandidatePaths()
		if candidates:
			_questDecisionPath = candidates[0]

	return _questDecisionPath


def getQuestDecisionResponsePath():
	global _questDecisionResponsePath

	if _questDecisionResponsePath is not None:
		return _questDecisionResponsePath

	decisionPath = getQuestDecisionPath()
	if decisionPath:
		_questDecisionResponsePath = os.path.join(
			os.path.dirname(decisionPath),
			"AgesBeyondQuestDecisionResponses.tsv")
		return _questDecisionResponsePath

	candidates = getQuestDecisionResponseCandidatePaths()
	if candidates:
		_questDecisionResponsePath = candidates[0]

	return _questDecisionResponsePath


def getQuestJournalPath():
	global _questJournalPath

	if _questJournalPath is not None and os.path.exists(_questJournalPath):
		return _questJournalPath

	for path in getQuestJournalCandidatePaths():
		if os.path.exists(path):
			_questJournalPath = path
			writeProbe("using quest journal file: %s" % path)
			return _questJournalPath

	if _questJournalPath is None:
		candidates = getQuestJournalCandidatePaths()
		if candidates:
			_questJournalPath = candidates[0]

	return _questJournalPath


def getCandidatePaths():
	return getCandidatePathsFor("AgesBeyondNotifications.tsv")


def getQuestCandidatePaths():
	return getCandidatePathsFor("AgesBeyondQuestNotifications.tsv")


def getQuestDecisionCandidatePaths():
	return getCandidatePathsFor("AgesBeyondQuestDecisions.tsv")


def getQuestDecisionResponseCandidatePaths():
	return getCandidatePathsFor("AgesBeyondQuestDecisionResponses.tsv")


def getQuestJournalCandidatePaths():
	return getCandidatePathsFor("AgesBeyondQuestJournal.tsv")


def getCandidatePathsFor(fileName):
	candidates = []

	def add(path):
		if path and path not in candidates:
			candidates.append(path)

	root = os.getcwd()
	add(os.path.join(root, "Mods", "Ages Beyond", "Chronicle", fileName))
	add(os.path.join(root, "Chronicle", fileName))

	try:
		here = os.path.dirname(os.path.abspath(__file__))
		add(os.path.abspath(os.path.join(here, "..", "..", "Chronicle", fileName)))
	except:
		pass

	return candidates


def writeProbe(message):
	global _lastProbeMessage
	if message == _lastProbeMessage:
		return
	_lastProbeMessage = message

	try:
		for notificationPath in getCandidatePaths():
			chronicleDir = os.path.dirname(notificationPath)
			if os.path.isdir(chronicleDir):
				file = open(os.path.join(chronicleDir, "AgesBeyondPythonProbe.txt"), "ab")
				try:
					file.write("%s\n" % message)
				finally:
					file.close()
				return
	except:
		pass


def pollQuestJournal():
	global _questJournalSignature

	path = getQuestJournalPath()
	if not path or not os.path.exists(path):
		if not _missingNotificationChannels.get("quest-journal", False):
			_missingNotificationChannels["quest-journal"] = True
			writeProbe("missing quest journal file")
		return
	_missingNotificationChannels["quest-journal"] = False

	try:
		summary = readQuestJournalSummary(path)
		if summary is None:
			return

		signature = "\t".join(summary)
		if signature == _questJournalSignature:
			return

		activeCount = toInt(summary[1], 0)
		completedCount = toInt(summary[2], 0)
		if activeCount <= 0 and completedCount <= 0:
			_questJournalSignature = signature
			return

		text = "Quest Journal: %d active" % activeCount
		if completedCount > 0:
			text = "%s, %d completed" % (text, completedCount)
		headline = summary[3].strip()
		if headline and headline != "No active living quests":
			text = "%s. %s" % (text, headline)

		if showMessage(text):
			_questJournalSignature = signature
	except Exception, e:
		writeProbe("quest journal poll failed: %s" % e)
		print "AgesBeyondNotifications: failed to poll quest journal file"


def pollQuestDecisions():
	global _questDecisionOffset

	path = getQuestDecisionPath()
	if not path or not os.path.exists(path):
		if not _missingNotificationChannels.get("quest-decision", False):
			_missingNotificationChannels["quest-decision"] = True
			writeProbe("missing quest decision file")
		return
	_missingNotificationChannels["quest-decision"] = False

	try:
		size = os.path.getsize(path)
		if size < _questDecisionOffset:
			_questDecisionOffset = 0

		file = open(path, "rb")
		try:
			file.seek(_questDecisionOffset)
			while True:
				lineStart = file.tell()
				line = file.readline()
				if not line:
					break
				if showQuestDecisionLine(line):
					_questDecisionOffset = file.tell()
				else:
					file.seek(lineStart)
					break
		finally:
			file.close()
	except Exception, e:
		writeProbe("quest decision poll failed: %s" % e)
		print "AgesBeyondNotifications: failed to poll quest decision file"


def showQuestDecisionLine(line):
	if not canShowMessage():
		return False

	line = line.strip()
	if not line:
		return True

	parts = line.split("\t", 6)
	if len(parts) != 7:
		return True

	decisionId = parts[0].strip()
	playerId = toInt(parts[1], -1)
	title = parts[2].strip()
	body = parts[3].strip()
	choices = [parts[4].strip(), parts[5].strip(), parts[6].strip()]

	if playerId != CyGame().getActivePlayer():
		return True
	if not decisionId or isQuestDecisionMade(decisionId):
		return True

	showQuestDecisionPopup(decisionId, title, body, choices)
	return True


def showQuestDecisionPopup(decisionId, title, body, choices):
	global _nextDecisionPopupId
	popupId = _nextDecisionPopupId
	_nextDecisionPopupId += 1
	_pendingDecisionPopups[popupId] = (decisionId, choices)

	popupInfo = CyPopupInfo()
	popupInfo.setButtonPopupType(ButtonPopupTypes.BUTTONPOPUP_PYTHON)
	popupInfo.setPythonModule("AgesBeyondNotifications")
	popupInfo.setOnClickedPythonCallback("onQuestDecisionClicked")
	popupInfo.setData1(popupId)
	popupInfo.setText(u"%s\n\n%s" % (title, body))
	for choice in choices:
		if choice:
			popupInfo.addPythonButton(unicode(choice), "")
	popupInfo.addPopup(CyGame().getActivePlayer())
	writeProbe("shown quest decision %s" % decisionId)


def onQuestDecisionClicked(argsList):
	iButton, iPopupId, iData2, iData3, iFlags, szText, bOption1, bOption2 = argsList
	if not _pendingDecisionPopups.has_key(iPopupId):
		return 0
	decisionId, choices = _pendingDecisionPopups[iPopupId]
	del _pendingDecisionPopups[iPopupId]

	choice = "No stance"
	if iButton >= 0 and iButton < len(choices):
		choice = choices[iButton]
	markQuestDecision(decisionId, choice)
	writeQuestDecisionResponse(decisionId, choice)
	showMessage("Quest Decision: %s" % choice)
	return 0


def isQuestDecisionMade(decisionId):
	return _madeQuestDecisions.has_key(decisionId)


def markQuestDecision(decisionId, choice):
	_madeQuestDecisions[decisionId] = choice


def writeQuestDecisionResponse(decisionId, choice):
	path = getQuestDecisionResponsePath()
	if not path:
		return

	try:
		directory = os.path.dirname(path)
		if directory and not os.path.isdir(directory):
			os.makedirs(directory)
		file = open(path, "ab")
		try:
			file.write("%s\t%d\t%s\n" % (
				sanitizeTsvText(decisionId),
				CyGame().getActivePlayer(),
				sanitizeTsvText(choice)))
		finally:
			file.close()
	except Exception, e:
		writeProbe("quest decision response write failed: %s" % e)
		print "AgesBeyondNotifications: failed to write quest decision response"


def sanitizeTsvText(value):
	try:
		value = unicode(value)
	except:
		value = "%s" % value
	value = value.replace("\t", " ").replace("\r", " ").replace("\n", " ")
	value = " ".join(value.split())
	return value.encode("ascii", "replace")


def readQuestJournalSummary(path):
	file = open(path, "rb")
	try:
		while True:
			line = file.readline()
			if not line:
				break
			parts = line.strip().split("\t", 3)
			if len(parts) == 4 and parts[0] == "summary":
				return parts
	finally:
		file.close()

	return None


def showLine(line, showPrefix=False):
	global _shownCount

	line = line.strip()
	if not line:
		return True

	parts = line.split("\t", 3)
	if len(parts) != 4:
		return True

	text = parts[3].strip()
	if not text:
		return True

	if showPrefix and not text.startswith("Quest:"):
		text = "Quest: %s" % text

	return showMessage(text)


def showMessage(text):
	global _shownCount

	if not canShowMessage():
		return False

	CyInterface().addImmediateMessage(text, "AS2D_POSITIVE_DINK")

	_shownCount += 1
	writeProbe("shown notification %d" % _shownCount)
	return True


def canShowMessage():
	iPlayer = CyGame().getActivePlayer()
	if iPlayer < 0:
		return False

	try:
		if not CyGame().isFinalInitialized():
			return False
	except:
		pass

	return True


def toInt(value, default):
	try:
		return int(value)
	except:
		return default
