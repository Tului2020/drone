from connection import mav_connection

def set_rc_channel_pwm(channel_id, pwm=1500):
    """ Set RC channel pwm value
    Args:
        channel_id (TYPE): Channel ID
        pwm (int, optional): Channel pwm value 1100-1900
    """
    if channel_id < 1 or channel_id > 18:
        print("Channel does not exist.")
        return

    # Mavlink 2 supports up to 18 channels:
    # https://mavlink.io/en/messages/common.html#RC_CHANNELS_OVERRIDE
    rc_channel_values = [2000 for _ in range(9)]
    rc_channel_values[channel_id - 1] = pwm
    mav_connection.mav.rc_channels_override_send(
        mav_connection.target_system,                   # target_system
        mav_connection.target_component,                # target_component
        *rc_channel_values,                             # RC channel list, in microseconds.
    )

    print(mav_connection.recv_match(type='COMMAND_ACK', blocking=True).to_dict())



# Example: Set channel 1 to 1500 PWM
set_rc_channel_pwm(1, 1600)

