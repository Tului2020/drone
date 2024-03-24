from shell import board

board.feature_jump()

while True:
    try:
        data = board.get_attitude()
        pitch = data['angy']
        roll = data['angx']
        heading = data['heading']

        speed_motor_1 = int(pitch * 10)
        speed_motor_2 = pitch * 10
        speed_motor_3 = pitch * 10
        speed_motor_4 = pitch * 10
        print(speed_motor_1)
        board.set_motor_individual(speed_motor_1, speed_motor_2, speed_motor_3, speed_motor_4)

    except KeyboardInterrupt:
        board.set_motor(0)
        break
    except:
        pass

