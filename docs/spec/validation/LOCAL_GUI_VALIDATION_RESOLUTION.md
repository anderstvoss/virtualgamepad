# Local GUI validation

The demo is a reference consumer of the same public controller-native API used
by external applications. GUI validation must select a controller and Linux
target explicitly, display creation and commit errors without degradation,
edit typed state, commit complete frames, show typed reverse events, and expose
controller diagnostics.

The GUI is not a profile browser, planner, YAML loader, or support oracle.
Successful visual interaction does not replace prepared-host identity,
descriptor, report, reverse-output, and teardown checks for the selected
provider. See [HEADLESS_TEST_STRATEGY.md](HEADLESS_TEST_STRATEGY.md).
