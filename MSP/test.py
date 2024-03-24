from shell import board

while True:
    data = board.get_attitude()
    try:
        print(data)
    except:
        pass

