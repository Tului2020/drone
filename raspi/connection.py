import serial

ser = serial.Serial('/dev/ttyS0', 9600, timeout=0)
if not ser.is_open:
    ser.open()