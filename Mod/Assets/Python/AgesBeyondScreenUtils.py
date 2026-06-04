import CvScreenUtils
import AgesBeyondNotifications


class AgesBeyondScreenUtils(CvScreenUtils.CvScreenUtils):
	def __init__(self):
		self.updateCounter = 0
		AgesBeyondNotifications.writeProbe("screen utils initialized")

	def update(self, argsList):
		self.updateCounter += 1
		if self.updateCounter >= 15:
			self.updateCounter = 0
			try:
				AgesBeyondNotifications.writeProbe("screen utils polling")
				AgesBeyondNotifications.poll(0)
			except:
				print "Ages Beyond screen notification poll failed"

		return CvScreenUtils.CvScreenUtils.update(self, argsList)
