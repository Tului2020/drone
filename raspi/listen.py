import connection
import time

ser = connection.ser
message_header = '$M>'

message_array = []
message_size = 0

def parse_message(raw_message):
    global message_size
    global message_id
    global message_array
    message_parse_idx = len(message_array)

    for _byte in raw_message:
        print(_byte)
        if (message_parse_idx < 3):
            if (chr(_byte) == message_header[message_parse_idx]):
                message_array.append(chr(_byte))
                message_parse_idx += 1
                pass
            else:
                message_parse_idx = 0
                message_array = []
                message_size = 0
                pass
        if (message_parse_idx == 3):
            message_parse_idx += 1
            message_size = _byte
            message_array.append(_byte)
            pass
        if (message_parse_idx == 4):
            message_parse_idx += 1
            message_id = _byte
            message_array.append(_byte)
            pass
        if (message_parse_idx == 5):
            pass

try:
    while True:
        if ser.in_waiting > 0:
            received_data = ser.read(1)
            parse_message(received_data)

except KeyboardInterrupt:
    pass