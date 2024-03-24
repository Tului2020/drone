from shell import board
from pprint import pprint

# board.feature_jump()

def format_data(data):
    format_str = "{:0}: {:+5.2f}; "
    return_string = ""

    for key in ['ax', 'ay', 'az', 'gx', 'gy', 'gz', 'mx', 'my', 'mz']:
        try:
            return_string += format_str.format(key, data[key])
        except:
            pass
    
    return return_string

while True:
    print(format_data(board.get_imu()))
    # pprint(x['ax'])
    # # print(board.get_attitude())

    # # print(board.get_imu())
    # format_str = "{:2}: {:+7.2f}"

    # # Printing each item with consistent formatting
    # for key, value in data.items():
    #     print(format_str.format(key.upper(), value))

