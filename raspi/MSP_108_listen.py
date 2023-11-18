message = b'$M>\x06lq\xffq\x00\x0b\x00\x9e'
message = b'$M>\x06ll\xffe\x00\x00\x00\x9c'

message_array = []
for i in range(len(message)):
    message_array.append(message[i])
print(message_array)
