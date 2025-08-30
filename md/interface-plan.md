# Interface plan

## On startup, display settings

Check whether the app has been granted Accessibility preferences and whether we have recorded User Preferences. If either is false go open the settings dialog.

## Settings dialog

The settings dialog displays:

* Accessibility permissions
    * Either a green check and "Granted"
    * Or a red "X" and a button "Request"
        * it should say "Symposium requires accessibility permissions."
* Then it should offer a choice of modes, each accompanied by a representative screenshot drawn using rectangles:
    * Free-form
    * Stacked
* And it should have some preferences like
    * On connecting to a new agentspace start:
    * Communicate with the 
