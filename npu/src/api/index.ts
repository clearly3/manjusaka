import { defHttp } from "/@/utils/http/axios";
import { ErrorMessageMode } from "/#/axios";
import { UploadFileParams } from "/#/axios";


export function GetApi(data: Any, mode: ErrorMessageMode = "modal") {
  return defHttp.get<any>(
    {
      url: data.action,
      params: data.params,
    },
    {
      errorMessageMode: mode,
    }
  );
}


export function PostApi(
  data: Any,
  mode: ErrorMessageMode = "modal",
) {
  return defHttp.post<Any>(
    {
      url: data.action,
      params: data.data,
    },
    {
      errorMessageMode: mode,
    }
  );
}


export interface UploadApiResult {
  message: string;
  code: number;
  url: string;
}

export function UploadApi(
  params: UploadFileParams,
  onUploadProgress: (progressEvent: ProgressEvent) => void
) {
  return defHttp.uploadFile<UploadApiResult>(
    {
      url: params.action,
      onUploadProgress,
    },
    params
  );
}
