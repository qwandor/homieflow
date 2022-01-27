# Houseflow

Houseflow is a gateway between Google Home and [devices](docs/homie.md) such as lights and
temperature sensors following the [Homie convention](https://homieiot.github.io/) defined on top of
MQTT.

## Code organisation

The main modules and directories are:

### config/

Code for parsing the TOML configuration file.

### fulfillment/

Fulfillment service supports following intents

- Sync, get all available devices for a user.
- Query, used to check device state.
- Execute, used to execute some command on device, e.g turn on lights.

### oauth/

Handles OAuth2 requests from Google Home.

### extractors.rs

Axum extractors for things such as `RefreshToken` or `AccessToken`.

## License

Licensed under the GNU General Public License, Version 3 ([LICENSE](LICENSE) or https://www.gnu.org/licenses/gpl-3.0.en.html).

## Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, shall be licensed as above, without any additional terms or conditions.

If you want to contribute to the project, see details of
[how we accept contributions](CONTRIBUTING.md).
