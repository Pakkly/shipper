#! /bin/bash
APP_NAME="exampleapp"
cargo build --release
rm -rf ./target/dist/${APP_NAME}.app
APP_CONTENTS=./target/dist/${APP_NAME}.app/Contents
mkdir -p ${APP_CONTENTS}/MacOS
mkdir ${APP_CONTENTS}/Resources
cp target/release/exampleapp ${APP_CONTENTS}/MacOS
cp bundle_artifacts/MacOS/info.plist ${APP_CONTENTS}
codesign --deep --options runtime -s "Developer ID Application: Zan Vidrih (329CGDDU5U)" target/dist/${APP_NAME}.app -f --timestamp