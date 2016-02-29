package events

// This file was generated by the swagger tool.
// Editing this file might prove futile when you re-run the swagger generate command

import (
	"net/http"

	"github.com/go-swagger/go-swagger/httpkit"
)

/*PutEventByIDNoContent Successful response

swagger:response putEventByIdNoContent
*/
type PutEventByIDNoContent struct {
}

// NewPutEventByIDNoContent creates PutEventByIDNoContent with default headers values
func NewPutEventByIDNoContent() PutEventByIDNoContent {
	return PutEventByIDNoContent{}
}

// WriteResponse to the client
func (o *PutEventByIDNoContent) WriteResponse(rw http.ResponseWriter, producer httpkit.Producer) {

	rw.WriteHeader(204)
}

/*PutEventByIDDefault Generic Error

swagger:response putEventByIdDefault
*/
type PutEventByIDDefault struct {
}

// NewPutEventByIDDefault creates PutEventByIDDefault with default headers values
func NewPutEventByIDDefault() PutEventByIDDefault {
	return PutEventByIDDefault{}
}

// WriteResponse to the client
func (o *PutEventByIDDefault) WriteResponse(rw http.ResponseWriter, producer httpkit.Producer) {

	rw.WriteHeader(500)
}