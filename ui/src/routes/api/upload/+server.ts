import { json, type RequestHandler } from "@sveltejs/kit";
import { writeFileSync } from 'fs';

export const POST: RequestHandler = async function (event) {
    try {
        const formData = Object.fromEntries(await event.request.formData());

        if (

            !(formData.dopplerFile as File).name ||
            (formData.dopplerFile as File).name === 'undefined'
        ) {

            return json({
                error: 'Failed to upload doppler file'
            });
        }

        if (!(formData.dopplerFile as File).name.endsWith(".doppler")
        ) {
            return json({
                error: 'Only .doppler files can be uploaded'
            });
        }

        const { dopplerFile } = formData as { dopplerFile: File };

        writeFileSync(`./static/doppler_files/${dopplerFile.name}`, Buffer.from(await dopplerFile.arrayBuffer()));

        return json({
            "message": "successfully upload file"
        });
    } catch (error) {
        console.error(error);
        return json({
            error: 'Failed to upload doppler file'
        });
    }
};