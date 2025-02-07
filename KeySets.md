
## This file attempts to map profile keys to their Device Keys

#### LEDTree (Colours are BGRA format)
  `01,00,00` -> `u32` -> LEDMode  
  `01,01,00` -> `RGB` -> LEDcolour1   
  `01,02,00` -> `RGB` -> LEDcolour2  
  `-UNKNOWN` -> `RGB` -> LEDcolour3 - Suspect it's `03`, but not seen in traffic.  
  `01,04,00` -> `u32` -> LEDSpeed  
  `01,05,00` -> `u32` -> LEDBrightness (Percent)  
  `01,06,00` -> `u32` -> LEDmeterSource  
  `01,07,00` -> `f32` -> LEDmeterSens  
  `01,08,00` -> `u32` -> LEDmuteMode  
  `01,09,00` -> `RGB` -> LEDmuteColour  
  `01,0a,00` -> `???` -> Value returned is 0, no direct profile matches.  
  `01,0b,00` -> `u32` -> LEDsuspendMode  
  `01,0c,00` -> `u32` -> LEDsuspendBrightness  
  `-UNKNOWN` -> `RGB` -> MultiMicIDColour, Value Matches LEDcolour1
  
  
  