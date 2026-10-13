module.exports = {
  preset: '@react-native/jest-preset',
  setupFiles: ['<rootDir>/jest/qvpModule.js'],
  // The package is linked from ../qvp-react-native, which may have its own React: use the app's,
  // as metro.config.js does.
  moduleNameMapper: {
    '^react$': '<rootDir>/node_modules/react',
    '^react/(.*)$': '<rootDir>/node_modules/react/$1',
    '^react-native$': '<rootDir>/node_modules/react-native',
  },
};
