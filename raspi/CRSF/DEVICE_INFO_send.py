from connection import ser

# EXTENDED
msg_sync_byte = [0xC8]              # Sync Byte: To Flight Controller
msg_len = [0x04]                    # Frame Length: 4
msg_type = [0x28]                   # Message Type: Ping Device
msg_destination_address = [0x00]    # Destination Address: Broadcast Address
msg_origin_address = [0xEA]         # Origin Address: Radio Transmitter
msg_payload = [0x54, 0x2B]          # 
msg_crc = [0xC0]                    # Checksum

# ser.write(bytes(
#     [
#         *msg_sync_byte,
#         *msg_len,
#         *msg_type,
#         *msg_destination_address,
#         *msg_origin_address,
#         *msg_payload,
#         *msg_crc
#     ]
# ))

ser.write(bytes(
    [
        *[0xC8],
        *[0x04],
        *[0x28],
        *[0x00],
        *[0xEA],
        *[0x54, 0x2B],
        *[0xC0]
    ]
))


# print(checksum) # 194
#       EE 04 28 00 EA 54 2B C0
# HOST: EE 04 28 00 EA 54
# EE = dest
# 04 = len
# 28 = type
# 00 EA = extended packet
#         00 = CRSF_ADDRESS_BROADCAST (extended destination)
#         EA = CRSF_ADDRESS_RADIO_TRANSMITTER (extended source)
# 54 = CRC[]


# [238, 6, 44, 238, 234, 1, 0, 134]