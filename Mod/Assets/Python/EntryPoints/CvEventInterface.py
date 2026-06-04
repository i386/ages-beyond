# Sid Meier's Civilization 4
# Copyright Firaxis Games 2005

import CvEventManager
import AgesBeyondNotifications

eventManager = CvEventManager.CvEventManager()


def getEventManager():
	return eventManager


def onEvent(argsList):
	"""Called when a game event happens - return 1 if the event was consumed."""
	try:
		if argsList and argsList[0] in ("GameStart", "OnLoad"):
			AgesBeyondNotifications.reset()
		if argsList and argsList[0] == "gameUpdate":
			AgesBeyondNotifications.poll(argsList[1][0])
		else:
			AgesBeyondNotifications.poll(0)
	except:
		print "Ages Beyond notification poll failed"

	return getEventManager().handleEvent(argsList)


def applyEvent(argsList):
	return getEventManager().applyEvent(argsList)


def beginEvent(context, argsList=-1):
	return getEventManager().beginEvent(context, argsList)
