# For this to work, BetaFlight needs the following configration
# 1. Ports      -> Correct UART Port needs to be configured as "Configuration/MSP"
# 2. Receiver   -> Receiver Mode also needs to be configured for "MSP (control via MSP port)"

import serial

ser = serial.Serial('/dev/ttyS0', 115200, timeout=0.0001)
if not ser.is_open:
    ser.open()