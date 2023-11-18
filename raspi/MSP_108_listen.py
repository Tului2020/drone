message = b'$M>\x06lq\xffq\x00\x0b\x00\x9e'


request1 = b'$M<\x00ll'
request2 = b'$M>\x00ll'

message_array = []
for i in range(len(message)):
    message_array.append(message[i])
print(message_array)
print(len(message_array))