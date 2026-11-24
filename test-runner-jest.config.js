// test-storybook 기본 Jest 설정에서 빌드 산출물(설치본 .snap)을 스냅샷으로 오인하지 않게 제외한다.
import { getJestConfig } from '@storybook/test-runner';

export default { ...getJestConfig(), modulePathIgnorePatterns: ['<rootDir>/artifacts/', '<rootDir>/target/'] };
