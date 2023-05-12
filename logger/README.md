# Logger
This module is responsible for logging live vehicle data to the DB, managing it and making conclusions and inference from the logged data.

## Health Degradation
This section details how the health degradation for the vehicle is calculated when reports are generated

### General Algorithm
1. Define the number of engine stalling events required to cause a 1% degradation. Let's call this value `E`.
2. For each day, count the number of engine stalling events detected by your system. Let's call this count `S`.
3. Calculate the percentage degradation for that day using the following formula:

    ```
    Percentage Degradation = S / E * 0.01
    ```
    This formula calculates the ratio of the actual number of stalling events to the number required to cause a 1% degradation and expresses it as a percentage. For example, if `E` is 100 and `S` is 10, the percentage degradation is 0.1%.
4. Aggregate the percentage degradation values over a period of time (e.g., a week, a month) to get an overall estimate of the vehicle's health degradation during that period.
