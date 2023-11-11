from .connection import ser

message_count = 0
while True:
    message = input(f'{message_count}. message to be sent: ')
    ser.write(str.encode(message))
    message_count += 1