import serial

ser = serial.Serial('/dev/ttyS0', 416666, timeout=2)
if not ser.is_open:
    ser.open()