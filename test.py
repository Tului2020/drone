from connection import ser

message_segment = bytearray()

while True:
    if ser.in_waiting > 0:
        received_data = ser.read(1)
        if (received_data[0] == 0xC8):
            int_array = []
            for byte in message_segment:
                int_array.append(byte)
            print('message', int_array)
            message_segment.clear()
            message_segment.append(received_data[0])
        else:
            message_segment.append(received_data[0])

