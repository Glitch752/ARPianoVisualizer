# AR Piano Visualizer

A work-in-progress MIDI keyboard visualizer that uses AR to display the notes being played in the real world.

Uses ArUco fiducial markers (specifically 8.25cm AprilTag 25h9) to track the position of the keyboard.

## Setup
(Proper instructions coming soon)
- Print the existing AprilTag markers in `resources/aprilTags.pdf` on 8.5"x11" paper or generate your own [here](https://shiqiliu-67.github.io/apriltag-generator/) and cut them out.
  If the PDF appears blurry, try printing the png file.
- Attach the markers to your MIDI keyboard as shown in `resources/keyboard_markers.png`. Make sure they're as flat as possible.
  You can change the markers in code, but they must stay coplanar and, in the current implementation, in a line.
- Calibrate your device camera to get the correct camera matrix and distortion coefficients.
  Since this is a phone camera, you can do this using [CalibDB.net](https://calibdb.net/)
  If CalibDB says your camera already has calibration data available, you can download it and use it directly.
  This is the case for many phone cameras.
- Put the calibration data in `assets/calibration.json`