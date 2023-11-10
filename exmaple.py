import sys
from msp.multiwii import MultiWii

if __name__ == "__main__":
    try:
        print_debug = True
        fc = MultiWii("/dev/ttyS0", print_debug)
        fc.start()
        while True:
            print(fc.get_attitude())

    except Exception as error:
        import traceback
        print("Error on Main: " + str(error))
        traceback.print_exc()