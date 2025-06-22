const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const CopyPlugin = require('copy-webpack-plugin');

module.exports = {
  entry: './index.js',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'bundle.js',
    clean: true,
  },
  mode: 'production',
  plugins: [
    new HtmlWebpackPlugin({
      template: './index.html',
      inject: 'body',
      minify: false,
      scriptLoading: 'blocking' // 'defer' から 'blocking' に変更
    }),
    new CopyPlugin({
      patterns: [
        { from: 'pkg', to: 'pkg' }
      ],
    }),
  ],
  experiments: {
    asyncWebAssembly: true
  },
  devServer: {
    static: {
      directory: path.join(__dirname, 'dist'),
    },
    compress: true,
    port: 8080,
  },
};