// The ?raw is needed to get the raw svg string, otherwise it will be the file path
import moon from './svg/moon.svg?raw';
import sun from './svg/sun.svg?raw';
import copy from './svg/copy.svg?raw';
import check from './svg/check.svg?raw';
import radar from './svg/radar.svg?raw';
import octocat from './svg/octocat.svg?raw';
import download from './svg/download.svg?raw';

const icons = {
	moon,
	sun,
	copy,
	check,
	radar,
	octocat,
	download,
};

export type IconName = keyof typeof icons;
export default icons;
