// File is a CommonJS module; it may be converted to an ES module.
// NOTE: while ES modules are the future of JavaScript, many projects still
// use CommonJS for various reasons
// - going to ignore and stick with CommonJS for backwards compatibility
// import utilities for working with file and directory paths
const path = require("path");
// imports plugin to simplify creation of HTML files to serve webpack bundles
const HtmlWebpackPlugin = require("html-webpack-plugin");
// pluin to copy individual files or entire directories to  build directory
const CopyWebpackPlugin = require("copy-webpack-plugin");

module.exports = {
  // starts webpack configuration object to be exported
  entry: "./index.js", // webpack will start bundling from this file
  mode: "development", // enables certain builtin optimizations
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bundle.js",
  },
  experiments: {
    asyncWebAssembly: true,
  },
  module: {
    // rules using webpack's built-in Asset Modules to process these files
    rules: [
      {
        test: /\.(png|svg|jpg|jpeg|gif)$/i,
        type: "asset/resource",
      },
    ],
  },
  plugins: [
    // generates an html file for our wasm application and automatically
    // include the bundled Javascript file ('bundle.js' @output.filename)
    new HtmlWebpackPlugin({
      template: "./index.html",
    }),
    // plugin will copy files from '../static' directory to output directory
    new CopyWebpackPlugin({
      patterns: [{ from: "../static", to: "" }],
    }),
  ],
};
