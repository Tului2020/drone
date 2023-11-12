import serial
import struct

ser = serial.Serial('/dev/ttyS0', 115200)

string = struct.pack('cccBBB', '$', 'M', '<', 0, 100, 100)  # simple MSP_IDENT
print(ser.write(string))

