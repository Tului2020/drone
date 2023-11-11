import serial

ser = serial.Serial('/dev/ttyS0', 115200, timeout=0)
if not ser.is_open:
    ser.open()

try:
    while True:
        
        if ser.in_waiting > 0:
            data = ser.readline()
            print(data)

except KeyboardInterrupt:
    pass