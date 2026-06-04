from CvPythonExtensions import *
import os

gc = CyGlobalContext()

_notificationOffset = 0
_notificationPath = None


def reset():
	global _notificationOffset
	_notificationOffset = 0


def poll(iUpdate=0):
	if iUpdate % 10 != 0:
		return

	global _notificationOffset
	path = getNotificationPath()
	if not path or not os.path.exists(path):
		return

	try:
		size = os.path.getsize(path)
		if size < _notificationOffset:
			_notificationOffset = 0

		file = open(path, "rb")
		try:
			file.seek(_notificationOffset)
			for line in file.readlines():
				showLine(line)
			_notificationOffset = file.tell()
		finally:
			file.close()
	except:
		print "AgesBeyondNotifications: failed to poll notification file"


def getNotificationPath():
	global _notificationPath

	if _notificationPath is None:
		root = os.getcwd()
		_notificationPath = os.path.join(root, "Mods", "Ages Beyond", "Chronicle", "AgesBeyondNotifications.tsv")

	return _notificationPath


def showLine(line):
	line = line.strip()
	if not line:
		return

	parts = line.split("\t", 3)
	if len(parts) != 4:
		return

	text = parts[3].strip()
	if not text:
		return

	iPlayer = CyGame().getActivePlayer()
	if iPlayer < 0:
		return

	iX = toInt(parts[1], -1)
	iY = toInt(parts[2], -1)
	bHasPlot = iX >= 0 and iY >= 0

	CyInterface().addMessage(
		iPlayer,
		False,
		gc.getEVENT_MESSAGE_TIME(),
		"Ages Beyond: %s" % text,
		"AS2D_POSITIVE_DINK",
		InterfaceMessageTypes.MESSAGE_TYPE_MAJOR_EVENT,
		None,
		gc.getInfoTypeForString("COLOR_HIGHLIGHT_TEXT"),
		iX,
		iY,
		bHasPlot,
		bHasPlot)


def toInt(value, default):
	try:
		return int(value)
	except:
		return default
