import init, { greet } from "../pkg/alligator.js";

async function main() {
    await init();

    greet("alligator in the browser");
}

main();
