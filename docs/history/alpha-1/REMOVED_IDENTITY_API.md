
Historical constructor-removal regression (current creation uses family options):

```compile_fail,E0432
use virtualgamepad::{create_dualsense_with_identity, create_dualshock4_with_identity};
```
