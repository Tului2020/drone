from connection import mav_connection

def main():
    print("Waiting for MAVLink messages...")
    while True:
        try:
            # Wait for a valid MAVLink message
            # msg = mav_connection.recv_match(type='COMMAND_ACK', blocking=True)
            msg = mav_connection.recv_match(type='ATTITUDE', blocking=True)
            if msg:
                msg_dict = msg.to_dict()
                print(f"ROLL:   {msg_dict['roll']}")
                print(f"PITCH:  {msg_dict['pitch']}")
                print(f"YAW:    {msg_dict['yaw']}")
                print()
        except Exception as e:
            print(f"Error: {e}")
            break

if __name__ == "__main__":
    main()
