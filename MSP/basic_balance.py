from shell import board

print("Drone service starting...")

multiplier = 20

while True:
    try:
        data = board.get_attitude()

        pitch = int(data['angy'])
        roll = int(data['angx'])
        heading = int(data['heading'])

        speed_motor_1 = 1000
        speed_motor_2 = 1000
        speed_motor_3 = 1000
        speed_motor_4 = 1000

        if pitch > 0:
            speed_motor_1 += pitch * multiplier
            speed_motor_3 += pitch * multiplier
        else:
            speed_motor_2 += -pitch * multiplier
            speed_motor_4 += -pitch * multiplier

        if roll > 0:
            speed_motor_1 += roll * multiplier
            speed_motor_2 += roll * multiplier
        else:
            speed_motor_3 += -roll * multiplier
            speed_motor_4 += -roll * multiplier
        board.set_motor_individual(speed_motor_1, speed_motor_2, speed_motor_3, speed_motor_4)
    except KeyboardInterrupt:
        board.set_motor(0)
        break
    except:
        pass

