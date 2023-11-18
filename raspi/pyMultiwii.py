from pymultiwii import MultiWii

serialPort = "/dev/tty.usbmodem0x80000001"
board = MultiWii(serialPort)
while True:
    print(board.getData(MultiWii.ATTITUDE))