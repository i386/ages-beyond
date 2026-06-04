from CvPythonExtensions import *
import os

gc = CyGlobalContext()

_notificationOffset = 0
_notificationPath = None
_lastProbeMessage = None
_shownCount = 0


def reset():
	global _notificationOffset
	_notificationOffset = 0
	writeProbe("reset")


def poll(iUpdate=0):
	if iUpdate % 5 != 0:
		return

	global _notificationOffset
	path = getNotificationPath()
	if not path or not os.path.exists(path):
		writeProbe("missing notification file")
		return

	try:
		size = os.path.getsize(path)
		if size < _notificationOffset:
			_notificationOffset = 0

		file = open(path, "rb")
		try:
			file.seek(_notificationOffset)
			while True:
				lineStart = file.tell()
				line = file.readline()
				if not line:
					break
				if showLine(line):
					_notificationOffset = file.tell()
				else:
					file.seek(lineStart)
					break
		finally:
			file.close()
	except Exception, e:
		writeProbe("poll failed: %s" % e)
		print "AgesBeyondNotifications: failed to poll notification file"


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


def getCandidatePaths():
	candidates = []

	def add(path):
		if path and path not in candidates:
			candidates.append(path)

	root = os.getcwd()
	add(os.path.join(root, "Mods", "Ages Beyond", "Chronicle", "AgesBeyondNotifications.tsv"))
	add(os.path.join(root, "Chronicle", "AgesBeyondNotifications.tsv"))

	try:
		here = os.path.dirname(os.path.abspath(__file__))
		add(os.path.abspath(os.path.join(here, "..", "..", "Chronicle", "AgesBeyondNotifications.tsv")))
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


def showLine(line):
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

	iPlayer = CyGame().getActivePlayer()
	if iPlayer < 0:
		return False

	try:
		if not CyGame().isFinalInitialized():
			return False
	except:
		pass

	CyInterface().addImmediateMessage(text, "AS2D_POSITIVE_DINK")

	_shownCount += 1
	writeProbe("shown notification %d" % _shownCount)
	return True


def toInt(value, default):
	try:
		return int(value)
	except:
		return default
