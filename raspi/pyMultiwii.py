from pymultiwii import MultiWii

serialPort = "/dev/ttyS0"
board = MultiWii(serialPort)
while True:
    print("getting board data...")
    print(board.getData(MultiWii.ATTITUDE))