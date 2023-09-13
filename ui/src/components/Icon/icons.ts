// The ?raw is needed to get the raw svg string, otherwise it will be the file path
import moon from './svg/moon.svg?raw';
import sun from './svg/sun.svg?raw';

const icons = {
	moon,
	sun
};

export type IconName = keyof typeof icons;
export default icons;
