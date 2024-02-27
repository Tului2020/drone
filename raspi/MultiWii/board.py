from msp import MultiWii

print("Connecting to Flight Controller")
board = MultiWii("/dev/ttyS0")
print("Flight Controller connected!")
