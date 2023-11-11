import serial

ser = serial.Serial('/dev/ttyS0', 115200, timeout=0.1)
if not ser.is_open:
    ser.open()