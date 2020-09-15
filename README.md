# Live Traffic for VATSIM Clients
This program brings the real world into VRC and any other server-configurable clients for VATSIM. It offers data pulled from flightradar24, and flightplans from flightaware.

## Preview
<details>
<summary>Videos</Summary>

Boston Clearance/Ground/Tower

[![](https://img.youtube.com/vi/hU109JQMo9Y/0.jpg)](https://www.youtube.com/watch?v=hU109JQMo9Y)]

Boston Center

[![](https://img.youtube.com/vi/khF5jed41oI/0.jpg)](https://www.youtube.com/watch?v=khF5jed41oI)]

</details>

## Installing
1. Grab the latest [release](https://github.com/Sequal32/vrcliveatc/releases/latest) and unzip to a directory of your choice.
2. Install [Microsoft Visual C++ Redistributable](https://www.microsoft.com/en-us/download/details.aspx?id=52685).
3. Configure values to your liking as described below.
4. For VRC:
    Open or create `myservers.txt` in Documents/VRC. Add the following entry: 
    ```
    127.0.0.1 LIVE TRAFFIC
    ```
5. Start `liveatc.exe`
6. Connect using the new server.
7. Optional: Listen to [LiveATC](https://www.liveatc.net/)

## Configuration
`config.json` is read by the program and can be configured as follows:
```
{
    "upper_lat": 42.48,     - The latitude of the upper left map bound
    "upper_lon": -71.28,    - The longitude of the upper left map bound
    "bottom_lat": 42.26,    - The latitude of the lower right map bound
    "bottom_lon": -70.74,   - The longitude of the lower right map bound

    "floor": 0,             - Aircraft below this altitude (in feet) will not be displayed
    "ceiling": 99999,       - Aircraft above this altitude will not be displayed
"callsign": "BOS_GND"       - The callsign you're connecting with. This is used for assigning squawk codes or tracking aircraft for fun!
}
```
