# Live Traffic for VATSIM Clients
This program brings the real world into VRC and any other server-configurable clients for VATSIM. It offers data pulled from FlightRadar24 and ADSBExchange, flightplans from FlightAware, and metars from the NOAA.

## Preview
<details>
<summary>Videos</Summary>

Boston Clearance/Ground/Tower

[![](https://img.youtube.com/vi/hU109JQMo9Y/0.jpg)](https://www.youtube.com/watch?v=hU109JQMo9Y)]

Boston Center

[![](https://img.youtube.com/vi/khF5jed41oI/0.jpg)](https://www.youtube.com/watch?v=khF5jed41oI)]

</details>

## Installing
1. Download the latest [release](https://github.com/Sequal32/vrcliveatc/releases/latest) and unzip to a directory of your choice.
2. Install [Microsoft Visual C++ Redistributable](https://www.microsoft.com/en-us/download/details.aspx?id=52685).
3. Configure values to your liking as described in [Configuration](#configuration).
4. For VRC:
    Open or *create* `myservers.txt` in `Documents/VRC`. Add the following entry: 
    ```
    127.0.0.1 LIVE TRAFFIC
    ```
   For Euroscope:
   Open or *create* `myipaddr.txt` in `Documents/EuroScope`. Add the following entry: 
    ```
    127.0.0.1 LIVE-TRAFFIC
    ```
5. Start `livetraffic.exe`
6. Connect using the new server in VRC/Euroscope.
7. Optional: Listen to [LiveATC](https://www.liveatc.net/)

## Notes

* Flightplans from FlightAware are disabled by default to avoid users from getting IP banned. Please use a range of no more than 100nm if you plan on turning this feature on. See [Configuration](#configuration) for how to enable it.
* Flightplans from FlightAware will only be pulled for airline flights with a callsign starting with three letters followed by numbers in order to limit requests.
* Sometimes ADSBExchange data will go beyond the range you defined in the config file. In this case, departure/arrival data from FlightRadar24 will not be reflected in those aircraft.

## Configuration
`config.json` is read by the program and can be configured as follows:
```
{
    "airport": "KBOS",          - The airport to view aircraft at
    "delay": 0,                 - How much time to buffer before displaying aircraft, useful for syncing with LiveATC
    "range": 30,                - How far away from the airport (in miles) aircraft should be shown
    "floor": 0,                 - Aircraft below this altitude (in feet) will not be processed
    "ceiling": 99999,           - Aircraft above this altitude will not be processed
    "use_flightaware": false    - Whether to pull flight plans from flightaware. You can disable this if you experience IP limits.
}
```
