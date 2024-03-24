from shell import board
import argparse

print("Drone service starting...")

parser = argparse.ArgumentParser(description='Speed multiplier')
parser.add_argument('mult', type=int, help='Speed multiplier', default=1, required=False)
args = parser.parse_args()

multiplier = args.mult

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

