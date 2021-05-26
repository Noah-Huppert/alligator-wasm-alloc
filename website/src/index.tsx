import "regenerator-runtime/runtime"; // to fix a bug w Parcel: https://flaviocopes.com/parcel-regeneratorruntime-not-defined/

import React from "react";
import ReactDOM from "react-dom";

import App from "./App";

ReactDOM.render(<App />, document.getElementById("app"));
