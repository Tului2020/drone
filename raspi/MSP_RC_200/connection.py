import serial

ser = serial.Serial('/dev/ttyS0', 115200, timeout=0.0001)
if not ser.is_open:
    ser.open()