const path = require('path');
const { getDefaultConfig, mergeConfig } = require('@react-native/metro-config');

// The library lives outside the app root (../qvp-react-native, symlinked into node_modules).
const lib = path.resolve(__dirname, '../qvp-react-native');
const escape = s => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const config = {
  watchFolders: [lib],
  resolver: {
    // A library checkout with its own node_modules (after `npm install` there, for its
    // typecheck) holds a second React. Metro must not see it: two Reacts break every hook.
    blockList: [new RegExp(`^${escape(lib)}/node_modules/.*`)],
    nodeModulesPaths: [path.resolve(__dirname, 'node_modules')],
    extraNodeModules: {
      react: path.resolve(__dirname, 'node_modules/react'),
      'react-native': path.resolve(__dirname, 'node_modules/react-native'),
    },
  },
};

module.exports = mergeConfig(getDefaultConfig(__dirname), config);
