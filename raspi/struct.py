import struct
import connection

ser = connection.ser

string = struct.pack('cccBBB', '$', 'M', '<', 0, 100, 100)  # simple MSP_IDENT
print(ser.write(string))

